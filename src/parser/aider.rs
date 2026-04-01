use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::model::{Conversation, Message, Role, Source};
use super::{ParseError, Parser};

/// Parser pour l'historique Aider.
///
/// Aider stocke ses conversations en Markdown dans :
/// - `~/.aider.chat.history.md` (global)
/// - `.aider.chat.history.md` (par projet, dans le working directory)
///
/// Format :
/// ```markdown
/// # aider chat started at 2025-10-01 19:24:08
/// > (output système, ignoré)
/// #### message user
/// réponse assistant
/// # aider chat started at 2025-10-01 19:25:43
/// ...
/// ```
///
/// Chaque `# aider chat started at <timestamp>` démarre une nouvelle session.
/// `#### <texte>` = message user.
/// Le reste (hors lignes `>`) = réponse assistant.
pub struct AiderParser;

impl AiderParser {
    fn default_paths() -> Vec<PathBuf> {
        let home = dirs_home();
        vec![home.join(".aider.chat.history.md")]
    }

    /// Parse le timestamp Aider : "2025-10-01 19:24:08"
    fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc())
    }
}

impl Parser for AiderParser {
    fn name(&self) -> &'static str {
        "Aider"
    }

    fn detect(&self) -> bool {
        Self::default_paths().iter().any(|p| p.exists())
    }

    fn scan(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let check = if paths.is_empty() {
            Self::default_paths()
        } else {
            // Pour les chemins custom, scanner récursivement
            let mut files = Vec::new();
            for root in paths {
                scan_recursive(root, &mut files);
            }
            return files;
        };

        check.into_iter().filter(|p| p.exists()).collect()
    }

    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError> {
        let content = fs::read_to_string(path).map_err(|e| ParseError::Io {
            path: path.clone(),
            source: e,
        })?;

        let sessions = parse_sessions(&content);

        if sessions.is_empty() {
            return Err(ParseError::Empty {
                path: path.clone(),
            });
        }

        // Fusionner toutes les sessions d'un fichier en une seule conversation
        // (un fichier .aider.chat.history.md = un projet)
        let mut all_messages = Vec::new();
        let mut first_ts: Option<DateTime<Utc>> = None;
        let mut last_ts: Option<DateTime<Utc>> = None;

        for session in &sessions {
            if first_ts.is_none() {
                first_ts = session.timestamp;
            }
            if session.timestamp.is_some() {
                last_ts = session.timestamp;
            }
            all_messages.extend(session.messages.clone());
        }

        if all_messages.is_empty() {
            return Err(ParseError::Empty {
                path: path.clone(),
            });
        }

        let now = Utc::now();
        let title = all_messages
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
            .unwrap_or_else(|| "Aider session".to_string());

        Ok(Conversation::new(
            title,
            Source::Aider,
            None,
            path.to_string_lossy().to_string(),
            first_ts.unwrap_or(now),
            last_ts.unwrap_or(now),
            all_messages,
        ))
    }
}

struct AiderSession {
    timestamp: Option<DateTime<Utc>>,
    messages: Vec<Message>,
}

fn parse_sessions(content: &str) -> Vec<AiderSession> {
    let mut sessions = Vec::new();
    let mut current_session: Option<AiderSession> = None;
    let mut current_role: Option<Role> = None;
    let mut current_text = String::new();

    for line in content.lines() {
        // Nouvelle session
        if let Some(ts_str) = line.strip_prefix("# aider chat started at ") {
            // Flush le message en cours
            if let Some(ref mut session) = current_session {
                flush_message(session, &mut current_role, &mut current_text);
                sessions.push(current_session.take().unwrap());
            }
            current_session = Some(AiderSession {
                timestamp: AiderParser::parse_timestamp(ts_str.trim()),
                messages: Vec::new(),
            });
            current_role = None;
            current_text.clear();
            continue;
        }

        // Ignorer si pas de session en cours
        let session = match current_session.as_mut() {
            Some(s) => s,
            None => continue,
        };

        // Lignes système (output de commandes) — ignorer
        if line.starts_with("> ") {
            continue;
        }

        // Message user (#### marque un message user sur une seule ligne)
        if let Some(user_text) = line.strip_prefix("#### ") {
            // Flush le message précédent (assistant ou user)
            flush_message(session, &mut current_role, &mut current_text);
            // Le user message est la ligne elle-même, on le flush immédiatement
            let trimmed = user_text.trim().to_string();
            if !trimmed.is_empty() {
                session.messages.push(Message::new(Role::User, trimmed, None));
            }
            // Ce qui suit sera de l'assistant
            current_role = None;
            current_text.clear();
            continue;
        }

        // Lignes non-#### après un message user = contenu assistant
        if !line.trim().is_empty() && current_role.is_none() {
            current_role = Some(Role::Assistant);
        }

        if current_role.is_some() {
            if !current_text.is_empty() {
                current_text.push('\n');
            }
            current_text.push_str(line);
        }
    }

    // Flush final
    if let Some(ref mut session) = current_session {
        flush_message(session, &mut current_role, &mut current_text);
        sessions.push(current_session.take().unwrap());
    }

    sessions
}

fn flush_message(
    session: &mut AiderSession,
    role: &mut Option<Role>,
    text: &mut String,
) {
    if let Some(r) = role.take() {
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            session
                .messages
                .push(Message::new(r, trimmed, None));
        }
    }
    text.clear();
}

fn scan_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Ignorer les dossiers cachés et node_modules
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    scan_recursive(&path, files);
                }
            } else if path
                .file_name()
                .is_some_and(|n| n == ".aider.chat.history.md")
            {
                files.push(path);
            }
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
