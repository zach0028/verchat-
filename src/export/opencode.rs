use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::model::{Conversation, Role};

/// Injecte une conversation dans OpenCode en écrivant directement
/// dans la base SQLite `~/.local/share/opencode/opencode.db`.
pub fn inject(conv: &Conversation) -> Result<String, String> {
    let db_path = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");

    if !db_path.exists() {
        return Err("OpenCode database not found".to_string());
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Cannot open OpenCode DB: {e}"))?;

    let now_ms = Utc::now().timestamp_millis();
    let session_id = format!("ses_{}", Uuid::new_v4().to_string().replace('-', "")[..24].to_string());
    let title = format!("⚡ {}", conv.title);

    // Créer la session
    conn.execute(
        "INSERT INTO session (id, project_id, slug, directory, title, version, time_created, time_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            session_id,
            "global",
            "verchat-import",
            "",
            title,
            "1.0.0",
            now_ms,
            now_ms,
        ],
    ).map_err(|e| format!("Cannot create session: {e}"))?;

    // Créer les messages et parts
    for msg in &conv.messages {
        let msg_id = format!("msg_{}", Uuid::new_v4().to_string().replace('-', "")[..24].to_string());
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        };

        let msg_data = json!({
            "role": role,
            "time": { "created": now_ms, "completed": now_ms },
            "modelID": conv.model.as_deref().unwrap_or("imported"),
            "providerID": "verchat",
            "mode": "build",
            "agent": "build",
            "path": { "cwd": "", "root": "/" },
            "cost": 0,
            "tokens": { "input": 0, "output": 0, "reasoning": 0, "cache": { "read": 0, "write": 0 } },
            "finish": "stop"
        });

        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                msg_id,
                session_id,
                now_ms,
                now_ms,
                msg_data.to_string(),
            ],
        ).map_err(|e| format!("Cannot create message: {e}"))?;

        // Créer le part (texte)
        let part_id = format!("prt_{}", Uuid::new_v4().to_string().replace('-', "")[..24].to_string());
        let part_data = json!({
            "type": "text",
            "text": msg.content,
        });

        conn.execute(
            "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                part_id,
                msg_id,
                session_id,
                now_ms,
                now_ms,
                part_data.to_string(),
            ],
        ).map_err(|e| format!("Cannot create part: {e}"))?;
    }

    Ok(session_id)
}
