use std::fs;
use std::path::PathBuf;

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour Windsurf IDE (Codeium).
///
/// Windsurf stocke ses conversations en Protobuf binaire dans
/// `~/.codeium/windsurf/cascade/*.pb`.
/// Le schema Protobuf n'est pas documenté — on extrait les strings UTF-8
/// lisibles comme pour Cursor.
pub struct WindsurfParser;

impl WindsurfParser {
    fn default_root() -> PathBuf {
        dirs_home().join(".codeium").join("windsurf").join("cascade")
    }
}

impl Parser for WindsurfParser {
    fn name(&self) -> &'static str {
        "Windsurf"
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
                    if path.extension().is_some_and(|e| e == "pb") && path.is_file() {
                        files.push(path);
                    }
                }
            }
        }
        files
    }

    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError> {
        let data = fs::read(path).map_err(|e| ParseError::Io {
            path: path.clone(),
            source: e,
        })?;

        let strings = extract_text_strings(&data);

        if strings.is_empty() {
            return Err(ParseError::Empty { path: path.clone() });
        }

        // Reconstituer les messages à partir des strings extraites
        let mut messages = Vec::new();
        let mut is_user = true;

        for s in &strings {
            let role = if is_user { Role::User } else { Role::Assistant };
            messages.push(Message::new(role, s.clone(), None));
            is_user = !is_user;
        }

        if messages.is_empty() {
            return Err(ParseError::Empty { path: path.clone() });
        }

        let title = messages
            .first()
            .map(|m| {
                let preview: String = m.content.chars().take(80).collect();
                if m.content.chars().count() > 80 { format!("{preview}...") } else { preview }
            })
            .unwrap_or_else(|| "Windsurf session".to_string());

        Ok(Conversation::new(
            title,
            Source::Windsurf,
            Some("windsurf-ai".to_string()),
            path.to_string_lossy().to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
            messages,
        ))
    }
}

/// Extrait les strings UTF-8 lisibles d'un blob Protobuf.
/// Filtre pour ne garder que du texte naturel (pas des chemins, des IDs, etc.).
fn extract_text_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = Vec::new();

    for &byte in data {
        if byte >= 32 && byte <= 126 || byte >= 0xC0 {
            current.push(byte);
        } else if byte >= 0x80 && byte <= 0xBF && !current.is_empty() {
            current.push(byte);
        } else {
            flush_string(&mut current, &mut strings);
        }
    }
    flush_string(&mut current, &mut strings);

    strings
}

fn flush_string(current: &mut Vec<u8>, strings: &mut Vec<String>) {
    if current.len() > 30 {
        if let Ok(s) = String::from_utf8(current.clone()) {
            // Garder seulement le texte naturel :
            // - Contient des espaces
            // - Plus d'un tiers de caractères alphabétiques
            // - N'est pas un chemin de fichier seul
            let has_spaces = s.contains(' ');
            let alpha_ratio = s.chars().filter(|c| c.is_alphabetic()).count() as f64 / s.len() as f64;
            let is_path = s.starts_with('/') && !s.contains(' ');

            if has_spaces && alpha_ratio > 0.3 && !is_path {
                strings.push(s);
            }
        }
    }
    current.clear();
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
