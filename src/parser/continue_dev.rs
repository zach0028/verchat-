use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour les sessions Continue.dev.
///
/// Continue.dev stocke un fichier JSON par session dans
/// `~/.continue/sessions/<sessionId>.json`.
pub struct ContinueDevParser;

#[derive(Deserialize)]
struct RawSession {
    title: Option<String>,
    history: Vec<RawHistoryItem>,
}

#[derive(Deserialize)]
struct RawHistoryItem {
    message: RawMessage,
}

#[derive(Deserialize)]
struct RawMessage {
    role: Option<String>,
    content: Option<RawContent>,
}

/// Le contenu peut être une string directe (assistant) ou un array de blocs (user).
#[derive(Deserialize)]
#[serde(untagged)]
enum RawContent {
    Text(String),
    Blocks(Vec<RawContentBlock>),
}

#[derive(Deserialize)]
struct RawContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
}

impl ContinueDevParser {
    fn default_root() -> PathBuf {
        dirs_home().join(".continue").join("sessions")
    }

    fn extract_text(content: &RawContent) -> String {
        match content {
            RawContent::Text(s) => s.clone(),
            RawContent::Blocks(blocks) => blocks
                .iter()
                .filter(|b| {
                    b.block_type
                        .as_deref()
                        .map_or(true, |t| t == "text")
                })
                .filter_map(|b| b.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl Parser for ContinueDevParser {
    fn name(&self) -> &'static str {
        "Continue.dev"
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
                    let path = entry.path();
                    // Ignorer sessions.json (c'est l'index, pas une session)
                    if path.extension().is_some_and(|ext| ext == "json")
                        && path.file_name().is_some_and(|n| n != "sessions.json")
                        && path.is_file()
                    {
                        files.push(path);
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

        let raw: RawSession = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => {
                return Err(ParseError::Empty {
                    path: path.clone(),
                })
            }
        };

        let mut messages = Vec::new();
        for item in &raw.history {
            let role = match item.message.role.as_deref() {
                Some("user") => Role::User,
                Some("assistant") => Role::Assistant,
                Some("system") => Role::System,
                _ => continue,
            };

            let text = match &item.message.content {
                Some(c) => Self::extract_text(c),
                None => continue,
            };

            if text.trim().is_empty() {
                continue;
            }

            messages.push(Message::new(role, text, None));
        }

        if messages.is_empty() {
            return Err(ParseError::Empty {
                path: path.clone(),
            });
        }

        let title = raw
            .title
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| generate_title(&messages));

        let now = chrono::Utc::now();

        Ok(Conversation::new(
            title,
            Source::ContinueDev,
            None, // Continue.dev ne stocke pas le modèle dans la session
            path.to_string_lossy().to_string(),
            now,
            now,
            messages,
        ))
    }
}

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
        .unwrap_or_else(|| "Untitled session".to_string())
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
