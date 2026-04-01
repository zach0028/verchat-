use std::path::PathBuf;

use chrono::{DateTime, TimeZone, Utc};

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour Cursor IDE.
///
/// Cursor stocke ses conversations dans `state.vscdb` (SQLite) dans la table
/// `cursorDiskKV` avec des blobs Protobuf sous les clés `agentKv:blob:*`.
///
/// Les blobs sont du Protobuf non documenté. On extrait les strings UTF-8
/// lisibles et on reconstitue les conversations à partir des patterns détectés.
pub struct CursorParser;

impl CursorParser {
    fn db_path() -> PathBuf {
        dirs_home()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join("User")
            .join("globalStorage")
            .join("state.vscdb")
    }
}

impl Parser for CursorParser {
    fn name(&self) -> &'static str {
        "Cursor"
    }

    fn detect(&self) -> bool {
        Self::db_path().exists()
    }

    fn scan(&self, _paths: &[PathBuf]) -> Vec<PathBuf> {
        let path = Self::db_path();
        if path.exists() { vec![path] } else { Vec::new() }
    }

    fn parse(&self, _path: &PathBuf) -> Result<Conversation, ParseError> {
        Err(ParseError::Empty { path: Self::db_path() })
    }
}

impl CursorParser {
    /// Parse les conversations depuis la DB Cursor.
    /// Extrait les strings des blobs Protobuf et reconstitue les conversations.
    pub fn parse_all(&self) -> Vec<Conversation> {
        let db_path = Self::db_path();
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

        // Lire tous les blobs de conversations
        let mut stmt = match conn.prepare(
            "SELECT key, value FROM cursorDiskKV WHERE key LIKE 'agentKv:blob:%' AND length(value) > 100",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let blobs: Vec<(String, Vec<u8>)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                ))
            })
            .ok()
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default();

        let mut conversations = Vec::new();

        for (key, blob) in &blobs {
            let strings = extract_strings(blob);
            if strings.is_empty() {
                continue;
            }

            // Heuristique : la première longue string est souvent le message user
            // Les strings suivantes sont des réponses ou du contexte
            let mut messages = Vec::new();
            let mut is_user = true;

            for s in &strings {
                if s.len() < 5 {
                    continue;
                }
                // Skip les chemins de fichiers et UUIDs
                if s.starts_with('/') && s.contains('/') && !s.contains(' ') {
                    continue;
                }
                if s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4 {
                    continue;
                }

                let role = if is_user { Role::User } else { Role::Assistant };
                messages.push(Message::new(role, s.clone(), None));
                is_user = !is_user;
            }

            if messages.is_empty() || messages.len() < 2 {
                continue;
            }

            let title = messages
                .first()
                .map(|m| {
                    let preview: String = m.content.chars().take(80).collect();
                    if m.content.chars().count() > 80 { format!("{preview}...") } else { preview }
                })
                .unwrap_or_else(|| "Cursor conversation".to_string());

            let hash = &key[key.len().saturating_sub(8)..];
            let (created, updated) = extract_timestamps(blob);
            let conv = Conversation::new(
                title,
                Source::Cursor,
                Some("cursor-ai".to_string()),
                format!("cursor://{hash}"),
                created,
                updated,
                messages,
            );

            conversations.push(conv);
        }

        conversations
    }
}

/// Extrait les strings UTF-8 lisibles d'un blob binaire (Protobuf).
/// Filtre pour ne garder que les strings qui ressemblent à du texte naturel.
fn extract_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = Vec::new();

    for &byte in data {
        if byte >= 32 && byte <= 126 || byte >= 0xC0 { // ASCII imprimable + début UTF-8
            current.push(byte);
        } else if byte >= 0x80 && byte <= 0xBF && !current.is_empty() { // continuation UTF-8
            current.push(byte);
        } else {
            if current.len() > 20 {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    // Garder seulement les strings qui contiennent des espaces (texte naturel)
                    if s.contains(' ') && s.chars().filter(|c| c.is_alphabetic()).count() > s.len() / 3 {
                        strings.push(s);
                    }
                }
            }
            current.clear();
        }
    }

    if current.len() > 20 {
        if let Ok(s) = String::from_utf8(current) {
            if s.contains(' ') && s.chars().filter(|c| c.is_alphabetic()).count() > s.len() / 3 {
                strings.push(s);
            }
        }
    }

    strings
}

/// Extrait les timestamps (created, updated) d'un blob protobuf Cursor.
/// Cherche des timestamps Unix 4 bytes (secondes) entre 2024 et 2027.
fn extract_timestamps(data: &[u8]) -> (DateTime<Utc>, DateTime<Utc>) {
    let min_ts: u32 = 1704067200; // 2024-01-01
    let max_ts: u32 = Utc::now().timestamp() as u32; // pas dans le futur
    let mut earliest: Option<u32> = None;
    let mut latest: Option<u32> = None;

    // Chercher seulement dans les premiers 200 bytes (les metadata protobuf sont au début)
    let search_len = data.len().min(200);
    for i in 0..search_len.saturating_sub(4) {
        let val = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        if val > min_ts && val < max_ts {
            match earliest {
                None => earliest = Some(val),
                Some(e) if val < e => earliest = Some(val),
                _ => {}
            }
            match latest {
                None => latest = Some(val),
                Some(l) if val > l => latest = Some(val),
                _ => {}
            }
        }
    }

    let now = Utc::now();
    let created = earliest
        .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
        .unwrap_or(now);
    let updated = latest
        .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
        .unwrap_or(now);

    (created, updated)
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
