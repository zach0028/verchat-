pub mod aider;
pub mod claude_code;
pub mod continue_dev;
pub mod lm_studio;

use std::path::PathBuf;

use crate::model::Conversation;

/// Erreurs possibles lors du parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("JSON parse error in {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("No messages found in {path}")]
    Empty { path: PathBuf },
}

/// Trait commun à tous les parsers.
///
/// Chaque outil IA (Claude Code, LM Studio, etc.) implémente ce trait.
/// Le Core appelle ces méthodes sans connaître les détails internes du format.
pub trait Parser {
    /// Nom lisible du parser (pour les logs et l'affichage).
    fn name(&self) -> &'static str;

    /// Détecte si l'outil est installé sur cette machine.
    fn detect(&self) -> bool;

    /// Scanne les chemins configurés et retourne les fichiers de conversations trouvés.
    fn scan(&self, paths: &[PathBuf]) -> Vec<PathBuf>;

    /// Parse un fichier de conversation et retourne une `Conversation`.
    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError>;
}
