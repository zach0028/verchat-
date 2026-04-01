use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde_json::json;

use crate::model::{Conversation, Role};

/// Injecte une conversation dans LM Studio en créant un fichier
/// `.conversation.json` dans `~/.lmstudio/conversations/`.
///
/// Le format doit matcher exactement ce que LM Studio attend :
/// - Tous les champs top-level requis (preset, systemPrompt, etc.)
/// - Messages user : type "singleStep" avec champ `preprocessed`
/// - Messages assistant : type "multiStep" avec `steps[].type = "contentBlock"`
pub fn inject(conv: &Conversation) -> Result<PathBuf, String> {
    let lm_dir = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .join(".lmstudio")
        .join("conversations");

    if !lm_dir.is_dir() {
        return Err("LM Studio conversations directory not found".to_string());
    }

    let messages: Vec<serde_json::Value> = conv
        .messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };

            match msg.role {
                Role::Assistant => {
                    // Format multiStep avec contentBlock (comme les vrais fichiers LM Studio)
                    json!({
                        "versions": [{
                            "type": "multiStep",
                            "role": "assistant",
                            "senderInfo": {
                                "senderName": conv.model.as_deref().unwrap_or("imported")
                            },
                            "steps": [
                                {
                                    "type": "status",
                                    "stepIdentifier": format!("{}-status", uuid::Uuid::new_v4()),
                                    "statusState": {
                                        "status": "done",
                                        "text": "Imported from VER.CHAT"
                                    }
                                },
                                {
                                    "type": "contentBlock",
                                    "stepIdentifier": format!("{}-content", uuid::Uuid::new_v4()),
                                    "content": [{
                                        "type": "text",
                                        "text": msg.content
                                    }]
                                }
                            ]
                        }]
                    })
                }
                _ => {
                    // Format singleStep avec preprocessed (requis par LM Studio)
                    json!({
                        "versions": [{
                            "type": "singleStep",
                            "role": role,
                            "content": [{
                                "type": "text",
                                "text": msg.content
                            }],
                            "preprocessed": {
                                "role": role,
                                "content": [{
                                    "type": "text",
                                    "text": msg.content
                                }]
                            }
                        }]
                    })
                }
            }
        })
        .collect();

    let now_ms = Utc::now().timestamp_millis();
    let model_name = conv.model.as_deref().unwrap_or("imported");

    // Format complet avec TOUS les champs que LM Studio attend
    let conversation = json!({
        "name": format!("⚡ {}", conv.title),
        "pinned": true,
        "createdAt": now_ms,
        "preset": "@local:focus",
        "tokenCount": 0,
        "userLastMessagedAt": now_ms,
        "assistantLastMessagedAt": now_ms,
        "systemPrompt": "",
        "messages": messages,
        "usePerChatPredictionConfig": false,
        "perChatPredictionConfig": {},
        "clientInput": "",
        "clientInputFiles": [],
        "userFilesSizeBytes": 0,
        "lastUsedModel": {
            "identifier": model_name,
            "indexedModelIdentifier": model_name,
            "instanceLoadTimeConfig": { "fields": [] },
            "instanceOperationTimeConfig": { "fields": [] }
        },
        "notes": format!("Imported from {} via VER.CHAT on {}", conv.source, Utc::now().format("%Y-%m-%d %H:%M")),
        "plugins": [],
        "pluginConfigs": {},
        "disabledPluginTools": [],
        "looseFiles": []
    });

    let filename = format!("{now_ms}.conversation.json");
    let filepath = lm_dir.join(&filename);

    let content = serde_json::to_string_pretty(&conversation)
        .map_err(|e| format!("JSON serialization error: {e}"))?;

    fs::write(&filepath, content)
        .map_err(|e| format!("Write error: {e}"))?;

    Ok(filepath)
}
