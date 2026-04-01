use std::fmt;
use serde::{Deserialize, Serialize};

/// Les outils IA supportés par VER.CHAT.
///
/// Chaque variante correspond à un parser spécifique
/// qui sait lire le format natif de l'outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    ClaudeCode,
    LmStudio,
    ContinueDev,
    Aider,
}

impl Source {
    /// Nom lisible pour l'affichage dans la TUI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::LmStudio => "LM Studio",
            Self::ContinueDev => "Continue.dev",
            Self::Aider => "Aider",
        }
    }

    /// Chemins par défaut où l'outil stocke ses conversations.
    /// Utilisé par la détection automatique lors du `verchat init`.
    pub fn default_paths(&self) -> Vec<&'static str> {
        match self {
            Self::ClaudeCode => vec!["~/.claude/projects/"],
            Self::LmStudio => vec!["~/.lmstudio/conversations/"],
            Self::ContinueDev => vec!["~/.continue/sessions/"],
            Self::Aider => vec![], // pas de chemin par défaut, dépend des projets
        }
    }

    /// Toutes les sources disponibles.
    pub fn all() -> &'static [Source] {
        &[
            Self::ClaudeCode,
            Self::LmStudio,
            Self::ContinueDev,
            Self::Aider,
        ]
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}
