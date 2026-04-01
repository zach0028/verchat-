use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour Gemini CLI.
///
/// Gemini CLI stocke ses sessions en JSON dans
/// `~/.gemini/tmp/<project_hash>/chats/session-*.json`.
/// Attention : les sessions expirent après 30 jours.
pub struct GeminiCliParser;

#[derive(Deserialize)]
struct RawSession {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "startTime")]
    start_time: Option<String>,
    #[serde(rename = "lastUpdated")]
    last_updated: Option<String>,
    messages: Vec<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    content: Option<String>,
    timestamp: Option<String>,
    model: Option<String>,
    tokens: Option<RawTokens>,
}

#[derive(Deserialize)]
struct RawTokens {
    input: Option<u64>,
    output: Option<u64>,
    cached: Option<u64>,
}

impl GeminiCliParser {
    fn default_root() -> PathBuf {
        dirs_home().join(".gemini").join("tmp")
    }

    fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
        s.parse::<DateTime<Utc>>().ok()
    }
}

impl Parser for GeminiCliParser {
    fn name(&self) -> &'static str {
        "Gemini CLI"
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
            // Scan ~/.gemini/tmp/*/chats/session-*.json
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let chats_dir = entry.path().join("chats");
                    if chats_dir.is_dir() {
                        if let Ok(chat_entries) = fs::read_dir(&chats_dir) {
                            for chat_entry in chat_entries.flatten() {
                                let path = chat_entry.path();
                                if path.extension().is_some_and(|e| e == "json") {
                                    files.push(path);
                                }
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

        let raw: RawSession = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => return Err(ParseError::Empty { path: path.clone() }),
        };

        let mut messages = Vec::new();
        let mut model_name: Option<String> = None;
        let mut total_tokens_input: u64 = 0;
        let mut total_tokens_cached: u64 = 0;
        let mut total_tokens_output: u64 = 0;

        for raw_msg in &raw.messages {
            let role = match raw_msg.msg_type.as_deref() {
                Some("user") => Role::User,
                Some("gemini") => Role::Assistant,
                Some("system") => Role::System,
                _ => continue,
            };

            let text = match &raw_msg.content {
                Some(t) if !t.trim().is_empty() => t.clone(),
                _ => continue,
            };

            let timestamp = raw_msg.timestamp.as_deref().and_then(Self::parse_timestamp);

            if model_name.is_none() {
                model_name = raw_msg.model.clone();
            }

            if let Some(tokens) = &raw_msg.tokens {
                total_tokens_input += tokens.input.unwrap_or(0);
                total_tokens_output += tokens.output.unwrap_or(0);
                total_tokens_cached += tokens.cached.unwrap_or(0);
            }

            messages.push(Message::new(role, text, timestamp));
        }

        if messages.is_empty() {
            return Err(ParseError::Empty { path: path.clone() });
        }

        let title = messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| {
                let preview: String = m.content.chars().take(80).collect();
                if m.content.chars().count() > 80 { format!("{preview}...") } else { preview }
            })
            .unwrap_or_else(|| "Gemini session".to_string());

        let created = raw.start_time.as_deref()
            .and_then(Self::parse_timestamp)
            .unwrap_or_else(Utc::now);
        let updated = raw.last_updated.as_deref()
            .and_then(Self::parse_timestamp)
            .unwrap_or(created);

        let mut conv = Conversation::new(
            title,
            Source::GeminiCli,
            model_name,
            path.to_string_lossy().to_string(),
            created,
            updated,
            messages,
        );

        if total_tokens_input > 0 || total_tokens_output > 0 {
            conv = conv.with_tokens(total_tokens_input, 0, total_tokens_cached, total_tokens_output);
        }

        Ok(conv)
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
