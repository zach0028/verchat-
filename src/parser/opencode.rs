use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour OpenCode.
///
/// OpenCode stocke ses données dans une base SQLite `~/.local/share/opencode/opencode.db`
/// avec les tables : session → message → part.
/// Le contenu texte est dans la table `part` avec `type: "text"`.
pub struct OpenCodeParser;

#[derive(Deserialize)]
struct MessageData {
    role: Option<String>,
    #[serde(rename = "modelID")]
    model_id: Option<String>,
    tokens: Option<TokenData>,
}

#[derive(Deserialize)]
struct TokenData {
    input: Option<u64>,
    output: Option<u64>,
    cache: Option<CacheData>,
}

#[derive(Deserialize)]
struct CacheData {
    read: Option<u64>,
    write: Option<u64>,
}

#[derive(Deserialize)]
struct PartData {
    #[serde(rename = "type")]
    part_type: Option<String>,
    text: Option<String>,
}

impl OpenCodeParser {
    fn default_db_path() -> PathBuf {
        dirs_home().join(".local").join("share").join("opencode").join("opencode.db")
    }

    fn millis_to_dt(ms: i64) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(ms).single().unwrap_or_else(Utc::now)
    }
}

impl Parser for OpenCodeParser {
    fn name(&self) -> &'static str {
        "OpenCode"
    }

    fn detect(&self) -> bool {
        Self::default_db_path().exists()
    }

    fn scan(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let db_path = if paths.is_empty() {
            Self::default_db_path()
        } else {
            paths.first().cloned().unwrap_or_else(Self::default_db_path)
        };

        if db_path.exists() {
            vec![db_path]
        } else {
            Vec::new()
        }
    }

    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError> {
        // OpenCode a une seule DB avec toutes les sessions.
        // On retourne une erreur car on parse par session, pas par fichier.
        // Le vrai parsing se fait dans parse_all.
        Err(ParseError::Empty { path: path.clone() })
    }
}

impl OpenCodeParser {
    /// Parse toutes les sessions depuis la DB OpenCode.
    /// Retourne un Vec de conversations (une par session).
    pub fn parse_all(&self) -> Vec<Conversation> {
        let db_path = Self::default_db_path();
        if !db_path.exists() {
            return Vec::new();
        }

        let conn = match rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        // Récupérer toutes les sessions
        let mut sessions_stmt = match conn.prepare(
            "SELECT id, title, time_created, time_updated FROM session ORDER BY time_created DESC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let sessions: Vec<(String, String, i64, i64)> = sessions_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .ok()
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default();

        let mut conversations = Vec::new();

        for (session_id, title, created_ms, updated_ms) in &sessions {
            // Récupérer les messages de cette session
            let mut msg_stmt = match conn.prepare(
                "SELECT m.id, m.data, p.data as part_data
                 FROM message m
                 LEFT JOIN part p ON p.message_id = m.id
                 WHERE m.session_id = ?1
                 ORDER BY m.time_created ASC, p.time_created ASC",
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let rows: Vec<(String, String, Option<String>)> = msg_stmt
                .query_map(rusqlite::params![session_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .ok()
                .map(|rows| rows.flatten().collect())
                .unwrap_or_default();

            let mut messages = Vec::new();
            let mut model_name: Option<String> = None;
            let mut total_input: u64 = 0;
            let mut total_output: u64 = 0;
            let mut total_cache_read: u64 = 0;
            let mut total_cache_write: u64 = 0;
            let mut last_msg_id = String::new();

            for (msg_id, msg_data_str, part_data_str) in &rows {
                // Parse message data for role and tokens
                if *msg_id != last_msg_id {
                    last_msg_id = msg_id.clone();
                    if let Ok(msg_data) = serde_json::from_str::<MessageData>(msg_data_str) {
                        if model_name.is_none() {
                            model_name = msg_data.model_id.clone();
                        }
                        if let Some(tokens) = &msg_data.tokens {
                            total_input += tokens.input.unwrap_or(0);
                            total_output += tokens.output.unwrap_or(0);
                            if let Some(cache) = &tokens.cache {
                                total_cache_read += cache.read.unwrap_or(0);
                                total_cache_write += cache.write.unwrap_or(0);
                            }
                        }
                    }
                }

                // Parse part data for text content
                if let Some(part_str) = part_data_str {
                    if let Ok(part) = serde_json::from_str::<PartData>(part_str) {
                        if part.part_type.as_deref() == Some("text") {
                            if let Some(text) = part.text {
                                if !text.trim().is_empty() {
                                    let role = serde_json::from_str::<MessageData>(msg_data_str)
                                        .ok()
                                        .and_then(|d| d.role)
                                        .map(|r| match r.as_str() {
                                            "user" => Role::User,
                                            "assistant" => Role::Assistant,
                                            _ => Role::System,
                                        })
                                        .unwrap_or(Role::User);

                                    messages.push(Message::new(role, text, None));
                                }
                            }
                        }
                    }
                }
            }

            if messages.is_empty() {
                continue;
            }

            let mut conv = Conversation::new(
                title.clone(),
                Source::OpenCode,
                model_name.clone(),
                format!("opencode://{session_id}"),
                Self::millis_to_dt(*created_ms),
                Self::millis_to_dt(*updated_ms),
                messages,
            );

            if total_input > 0 || total_output > 0 {
                conv = conv.with_tokens(total_input, total_cache_write, total_cache_read, total_output);
            }

            conversations.push(conv);
        }

        conversations
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
