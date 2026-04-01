use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::message::Message;
use super::source::Source;

/// Une conversation IA complète, importée depuis un outil.
///
/// C'est l'unité centrale de VER.CHAT : chaque conversation importée
/// est convertie vers cette struct, quel que soit l'outil d'origine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Identifiant unique de la conversation.
    pub id: Uuid,

    /// Titre de la conversation (extrait ou généré par le parser).
    pub title: String,

    /// Outil d'origine.
    pub source: Source,

    /// Modèle LLM utilisé (pas toujours disponible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Chemin du fichier source d'origine (pour l'import incrémental).
    pub source_path: String,

    /// Date de création de la conversation.
    pub created_at: DateTime<Utc>,

    /// Date de dernière modification.
    pub updated_at: DateTime<Utc>,

    /// Marqué comme favori par l'utilisateur.
    #[serde(default)]
    pub favorite: bool,

    /// Tags ajoutés par l'utilisateur.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Tokens input réguliers (plein tarif).
    #[serde(default)]
    pub tokens_input: u64,

    /// Tokens input écrits en cache (1.25x tarif).
    #[serde(default)]
    pub tokens_cache_write: u64,

    /// Tokens input lus depuis le cache (0.1x tarif).
    #[serde(default)]
    pub tokens_cache_read: u64,

    /// Tokens en sortie (réponses assistant).
    #[serde(default)]
    pub tokens_output: u64,

    /// Les messages de la conversation, dans l'ordre chronologique.
    pub messages: Vec<Message>,
}

impl Conversation {
    /// Crée une nouvelle conversation avec un UUID généré automatiquement.
    pub fn new(
        title: String,
        source: Source,
        model: Option<String>,
        source_path: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        messages: Vec<Message>,
    ) -> Self {
        // Estimer les tokens si pas fournis : ~1 token par 4 caractères
        let est_in: u64 = messages.iter()
            .filter(|m| m.role == super::message::Role::User || m.role == super::message::Role::System)
            .map(|m| (m.content.len() as u64) / 4)
            .sum();
        let est_out: u64 = messages.iter()
            .filter(|m| m.role == super::message::Role::Assistant)
            .map(|m| (m.content.len() as u64) / 4)
            .sum();

        Self {
            id: Uuid::new_v4(),
            title,
            source,
            model,
            source_path,
            created_at,
            updated_at,
            favorite: false,
            tags: Vec::new(),
            tokens_input: est_in,
            tokens_cache_write: 0,
            tokens_cache_read: 0,
            tokens_output: est_out,
            messages,
        }
    }

    /// Remplace par des tokens réels (fournis par la source).
    pub fn with_tokens(
        mut self,
        input: u64,
        cache_write: u64,
        cache_read: u64,
        output: u64,
    ) -> Self {
        self.tokens_input = input;
        self.tokens_cache_write = cache_write;
        self.tokens_cache_read = cache_read;
        self.tokens_output = output;
        self
    }

    /// Total tokens input (toutes catégories).
    pub fn tokens_in_total(&self) -> u64 {
        self.tokens_input + self.tokens_cache_write + self.tokens_cache_read
    }

    /// Total tokens (in + out).
    pub fn tokens_total(&self) -> u64 {
        self.tokens_in_total() + self.tokens_output
    }

    /// Nombre de messages dans la conversation.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Nombre de messages par rôle (pour les stats).
    pub fn message_count_by_role(&self, role: super::message::Role) -> usize {
        self.messages.iter().filter(|m| m.role == role).count()
    }
}
