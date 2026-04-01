use std::fs;
use std::path::PathBuf;

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
            let conv = Conversation::new(
                title,
                Source::Cursor,
                Some("cursor-ai".to_string()),
                format!("cursor://{hash}"),
                chrono::Utc::now(),
                chrono::Utc::now(),
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

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
