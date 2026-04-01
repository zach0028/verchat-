use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour les conversations Claude Code.
///
/// Claude Code stocke ses conversations en JSONL (une ligne JSON par événement)
/// dans `~/.claude/projects/<project-path-encodé>/<sessionId>.jsonl`.
pub struct ClaudeCodeParser;

// --- Structures intermédiaires pour désérialiser le JSONL ---
// On ne désérialise que les champs dont on a besoin (serde ignore le reste).

#[derive(Deserialize)]
struct RawEntry {
    #[serde(rename = "type")]
    entry_type: String,
    uuid: Option<String>,
    timestamp: Option<String>,
    message: Option<RawMessage>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
}

#[derive(Deserialize)]
struct RawMessage {
    content: Option<RawContent>,
    model: Option<String>,
    usage: Option<RawUsage>,
}

#[derive(Deserialize)]
struct RawUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

/// Le contenu d'un message Claude Code peut être :
/// - Une string directe (messages user)
/// - Un array de blocs (messages assistant : text, thinking, tool_use...)
#[derive(Deserialize)]
#[serde(untagged)]
enum RawContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

impl ClaudeCodeParser {
    /// Chemin par défaut des projets Claude Code.
    fn default_root() -> PathBuf {
        dirs_home().join(".claude").join("projects")
    }

    /// Extrait le texte d'un `RawContent`.
    /// - String directe → retourne telle quelle
    /// - Array de blocs → concatène tous les blocs `text`
    fn extract_text(content: &RawContent) -> String {
        match content {
            RawContent::Text(s) => s.clone(),
            RawContent::Blocks(blocks) => blocks
                .iter()
                .filter(|b| b.block_type == "text")
                .filter_map(|b| b.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// Essaie de parser un timestamp ISO 8601.
    fn parse_timestamp(ts: &str) -> Option<DateTime<Utc>> {
        ts.parse::<DateTime<Utc>>().ok()
    }

    /// Génère un titre à partir du premier message user.
    fn generate_title(messages: &[Message]) -> String {
        messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| {
                let preview: String = m.content.chars().take(80).collect();
                if m.content.len() > 80 {
                    format!("{preview}...")
                } else {
                    preview
                }
            })
            .unwrap_or_else(|| "Untitled conversation".to_string())
    }
}

impl Parser for ClaudeCodeParser {
    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self) -> bool {
        Self::default_root().is_dir()
    }

    fn scan(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let roots = if paths.is_empty() {
            vec![Self::default_root()]
        } else {
            paths.to_vec()
        };

        let mut files = Vec::new();
        for root in &roots {
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let project_dir = entry.path();
                    if !project_dir.is_dir() {
                        continue;
                    }
                    // Lire les .jsonl directement dans le dossier projet
                    // (on ignore le sous-dossier subagents/)
                    if let Ok(jsonl_entries) = fs::read_dir(&project_dir) {
                        for jsonl_entry in jsonl_entries.flatten() {
                            let path = jsonl_entry.path();
                            if path.extension().is_some_and(|ext| ext == "jsonl")
                                && path.is_file()
                            {
                                files.push(path);
                            }
                        }
                    }
                }
            }
        }
        files
    }

    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError> {
        let content = fs::read_to_string(path).map_err(|e| ParseError::Io {
            path: path.clone(),
            source: e,
        })?;

        let mut messages = Vec::new();
        let mut model: Option<String> = None;
        let mut first_timestamp: Option<DateTime<Utc>> = None;
        let mut last_timestamp: Option<DateTime<Utc>> = None;
        // Track tokens per requestId to deduplicate streaming chunks
        // (input, cache_write, cache_read, output)
        let mut request_tokens: HashMap<String, (u64, u64, u64, u64)> = HashMap::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Tolérant aux lignes malformées (fichiers écrits en streaming)
            let entry: RawEntry = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Filtrer les types de lignes
            let role = match entry.entry_type.as_str() {
                "user" => Some(Role::User),
                "assistant" => Some(Role::Assistant),
                "system" => Some(Role::System),
                _ => None,
            };

            let raw_msg = entry.message.as_ref();

            // Tokens : compter sur TOUTES les lignes assistant (avant le filtre texte)
            // car les chunks thinking/tool_use ont aussi un usage valide.
            // Dédupliquer par requestId : le streaming duplique l'usage, output_tokens augmente à chaque chunk.
            // On garde la dernière valeur (la plus grande) par requestId.
            if let Some(msg) = raw_msg {
                if let Some(usage) = &msg.usage {
                    let req_id = entry.request_id.clone()
                        .unwrap_or_else(|| entry.uuid.clone().unwrap_or_default());
                    request_tokens.insert(req_id, (
                        usage.input_tokens.unwrap_or(0),
                        usage.cache_creation_input_tokens.unwrap_or(0),
                        usage.cache_read_input_tokens.unwrap_or(0),
                        usage.output_tokens.unwrap_or(0),
                    ));
                }
            }

            // Pour les messages : extraire le texte, ignorer les chunks vides
            let role = match role {
                Some(r) => r,
                None => continue,
            };

            let raw_msg = match raw_msg {
                Some(m) => m,
                None => continue,
            };

            let text = match &raw_msg.content {
                Some(c) => Self::extract_text(c),
                None => continue,
            };

            if text.trim().is_empty() {
                continue;
            }

            // Parser le timestamp
            let timestamp = entry
                .timestamp
                .as_deref()
                .and_then(Self::parse_timestamp);

            if let Some(ts) = timestamp {
                if first_timestamp.is_none() || Some(ts) < first_timestamp {
                    first_timestamp = Some(ts);
                }
                if last_timestamp.is_none() || Some(ts) > last_timestamp {
                    last_timestamp = Some(ts);
                }
            }

            if model.is_none() {
                if let Some(m) = &raw_msg.model {
                    model = Some(m.clone());
                }
            }

            messages.push(Message::new(role, text, timestamp));
        }

        if messages.is_empty() {
            return Err(ParseError::Empty {
                path: path.clone(),
            });
        }

        let now = Utc::now();
        let title = Self::generate_title(&messages);

        // Tokens facturés par catégorie (SUM par requête dédupliquée)
        let total_input: u64 = request_tokens.values().map(|(i, _, _, _)| i).sum();
        let total_cache_write: u64 = request_tokens.values().map(|(_, cw, _, _)| cw).sum();
        let total_cache_read: u64 = request_tokens.values().map(|(_, _, cr, _)| cr).sum();
        let total_output: u64 = request_tokens.values().map(|(_, _, _, o)| o).sum();

        let mut conv = Conversation::new(
            title,
            Source::ClaudeCode,
            model,
            path.to_string_lossy().to_string(),
            first_timestamp.unwrap_or(now),
            last_timestamp.unwrap_or(now),
            messages,
        );

        if total_input > 0 || total_cache_write > 0 || total_cache_read > 0 || total_output > 0 {
            conv = conv.with_tokens(total_input, total_cache_write, total_cache_read, total_output);
        }

        Ok(conv)
    }
}

/// Retourne le home directory de l'utilisateur.
fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
