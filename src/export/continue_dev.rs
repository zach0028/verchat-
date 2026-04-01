use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::model::{Conversation, Role};

/// Injecte une conversation dans Continue.dev en créant un fichier
/// session `.json` dans `~/.continue/sessions/` et en mettant à jour l'index.
///
/// Format vérifié :
/// - Session : sessionId, title, workspaceDirectory, history[]
/// - History items user : message.content = [{type: "text", text: "..."}]  + contextItems, editorState, appliedRules
/// - History items assistant : message.content = "string directe" + contextItems, isGatheringContext
/// - Index sessions.json : [{sessionId, title, dateCreated, workspaceDirectory}]
pub fn inject(conv: &Conversation) -> Result<PathBuf, String> {
    let sessions_dir = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .join(".continue")
        .join("sessions");

    if !sessions_dir.is_dir() {
        return Err("Continue.dev sessions directory not found".to_string());
    }

    let session_id = Uuid::new_v4().to_string();
    let now_ms = Utc::now().timestamp_millis().to_string();

    // Construire l'historique au format exact Continue.dev
    let history: Vec<serde_json::Value> = conv
        .messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };

            let msg_id = Uuid::new_v4().to_string();

            match msg.role {
                Role::User => {
                    // User : content = array de blocks, avec contextItems et editorState
                    json!({
                        "message": {
                            "role": role,
                            "content": [{
                                "type": "text",
                                "text": msg.content
                            }],
                            "id": msg_id
                        },
                        "contextItems": [],
                        "editorState": null,
                        "appliedRules": []
                    })
                }
                _ => {
                    // Assistant/System : content = string directe
                    json!({
                        "message": {
                            "role": role,
                            "content": msg.content,
                            "id": msg_id
                        },
                        "contextItems": [],
                        "isGatheringContext": false
                    })
                }
            }
        })
        .collect();

    let title = format!("[VER.CHAT] {}", conv.title);

    let session = json!({
        "sessionId": session_id,
        "title": title,
        "workspaceDirectory": "",
        "history": history
    });

    let filepath = sessions_dir.join(format!("{session_id}.json"));

    let content = serde_json::to_string_pretty(&session)
        .map_err(|e| format!("JSON serialization error: {e}"))?;

    fs::write(&filepath, content)
        .map_err(|e| format!("Write error: {e}"))?;

    // Mettre à jour l'index sessions.json (format : [{sessionId, title, dateCreated, workspaceDirectory}])
    let index_path = sessions_dir.join("sessions.json");
    if index_path.exists() {
        if let Ok(index_content) = fs::read_to_string(&index_path) {
            if let Ok(mut index) = serde_json::from_str::<Vec<serde_json::Value>>(&index_content) {
                index.push(json!({
                    "sessionId": session_id,
                    "title": title,
                    "dateCreated": now_ms,
                    "workspaceDirectory": ""
                }));
                if let Ok(updated) = serde_json::to_string_pretty(&index) {
                    let _ = fs::write(&index_path, updated);
                }
            }
        }
    }

    Ok(filepath)
}
