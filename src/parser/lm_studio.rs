use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour les conversations LM Studio.
///
/// LM Studio stocke un fichier JSON par conversation dans
/// `~/.lmstudio/conversations/` (avec sous-dossiers par catégorie).
/// Les fichiers sont nommés `<timestamp>.conversation.json`.
pub struct LmStudioParser;

#[derive(Deserialize)]
struct RawConversation {
    name: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<u64>,
    #[serde(rename = "userLastMessagedAt")]
    user_last_messaged_at: Option<u64>,
    #[serde(rename = "lastUsedModel")]
    last_used_model: Option<RawModel>,
    #[serde(rename = "tokenCount")]
    token_count: Option<u64>,
    messages: Vec<RawMessage>,
}

#[derive(Deserialize)]
struct RawModel {
    identifier: Option<String>,
}

#[derive(Deserialize)]
struct RawMessage {
    versions: Vec<RawVersion>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum RawVersion {
    #[serde(rename = "singleStep")]
    SingleStep {
        role: String,
        content: Vec<RawContentBlock>,
    },
    #[serde(rename = "multiStep")]
    MultiStep {
        role: String,
        steps: Vec<RawStep>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct RawContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct RawStep {
    #[serde(rename = "type")]
    step_type: String,
    content: Option<Vec<RawContentBlock>>,
}

impl LmStudioParser {
    fn default_root() -> PathBuf {
        dirs_home().join(".lmstudio").join("conversations")
    }

    fn extract_text_from_version(version: &RawVersion) -> Option<(String, String)> {
        match version {
            RawVersion::SingleStep { role, content } => {
                let text: String = content
                    .iter()
                    .filter(|b| b.block_type == "text")
                    .filter_map(|b| b.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.trim().is_empty() {
                    None
                } else {
                    Some((role.clone(), text))
                }
            }
            RawVersion::MultiStep { role, steps } => {
                let text: String = steps
                    .iter()
                    .filter(|s| s.step_type == "contentBlock")
                    .flat_map(|s| s.content.as_deref().unwrap_or_default())
                    .filter(|b| b.block_type == "text")
                    .filter_map(|b| b.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n");
                if text.trim().is_empty() {
                    None
                } else {
                    Some((role.clone(), text))
                }
            }
            RawVersion::Unknown => None,
        }
    }

    fn millis_to_dt(ms: u64) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(ms as i64).single().unwrap_or_else(Utc::now)
    }
}

impl Parser for LmStudioParser {
    fn name(&self) -> &'static str {
        "LM Studio"
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
            scan_recursive(root, &mut files);
        }
        files
    }

    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError> {
        let content = fs::read_to_string(path).map_err(|e| ParseError::Io {
            path: path.clone(),
            source: e,
        })?;

        let raw: RawConversation = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => {
                return Err(ParseError::Empty {
                    path: path.clone(),
                })
            }
        };

        let mut messages = Vec::new();
        for raw_msg in &raw.messages {
            // Prendre la première version (la plus récente)
            if let Some(version) = raw_msg.versions.first() {
                if let Some((role_str, text)) = Self::extract_text_from_version(version) {
                    let role = match role_str.as_str() {
                        "user" => Role::User,
                        "assistant" => Role::Assistant,
                        "system" => Role::System,
                        _ => continue,
                    };
                    messages.push(Message::new(role, text, None));
                }
            }
        }

        if messages.is_empty() {
            return Err(ParseError::Empty {
                path: path.clone(),
            });
        }

        let title = raw
            .name
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| generate_title(&messages));

        let model = raw
            .last_used_model
            .and_then(|m| m.identifier);

        let created = raw.created_at.map(Self::millis_to_dt).unwrap_or_else(Utc::now);
        let updated = raw
            .user_last_messaged_at
            .map(Self::millis_to_dt)
            .unwrap_or(created);

        let mut conv = Conversation::new(
            title,
            Source::LmStudio,
            model,
            path.to_string_lossy().to_string(),
            created,
            updated,
            messages,
        );

        // LM Studio fournit un tokenCount total — réparti 50/50 (pas de cache)
        if let Some(tc) = raw.token_count {
            if tc > 0 {
                conv = conv.with_tokens(tc / 2, 0, 0, tc / 2);
            }
        }

        Ok(conv)
    }
}

/// Scanne récursivement un dossier pour trouver les `.conversation.json`.
fn scan_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_recursive(&path, files);
            } else if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".conversation.json"))
            {
                files.push(path);
            }
        }
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
        .unwrap_or_else(|| "Untitled conversation".to_string())
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
