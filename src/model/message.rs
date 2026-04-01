use std::fmt;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Le rôle de l'auteur d'un message dans une conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => f.write_str("User"),
            Self::Assistant => f.write_str("Assistant"),
            Self::System => f.write_str("System"),
        }
    }
}

/// Un message dans une conversation IA.
///
/// Représente un échange unique (user → assistant, system prompt, etc.).
/// Le `timestamp` est optionnel car certains outils (Aider) ne le fournissent pas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Identifiant unique du message.
    pub id: Uuid,

    /// Qui a écrit ce message.
    pub role: Role,

    /// Contenu textuel du message.
    pub content: String,

    /// Horodatage du message (absent pour certains outils).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl Message {
    /// Crée un nouveau message avec un UUID généré automatiquement.
    pub fn new(role: Role, content: String, timestamp: Option<DateTime<Utc>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp,
        }
    }
}
