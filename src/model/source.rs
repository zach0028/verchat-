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
    GeminiCli,
    OpenCode,
    Cursor,
    Windsurf,
}

impl Source {
    /// Nom lisible pour l'affichage dans la TUI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::LmStudio => "LM Studio",
            Self::ContinueDev => "Continue.dev",
            Self::Aider => "Aider",
            Self::GeminiCli => "Gemini CLI",
            Self::OpenCode => "OpenCode",
            Self::Cursor => "Cursor",
            Self::Windsurf => "Windsurf",
        }
    }

    /// Toutes les sources disponibles.
    pub fn all() -> &'static [Source] {
        &[
            Self::ClaudeCode,
            Self::LmStudio,
            Self::ContinueDev,
            Self::Aider,
            Self::GeminiCli,
            Self::OpenCode,
            Self::Cursor,
            Self::Windsurf,
        ]
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}
