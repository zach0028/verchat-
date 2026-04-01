#![allow(dead_code)]

pub mod aider;
pub mod claude_code;
pub mod continue_dev;
pub mod cursor;
pub mod gemini_cli;
pub mod lm_studio;
pub mod opencode;
pub mod windsurf;

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
pub trait Parser {
    fn name(&self) -> &'static str;
    fn detect(&self) -> bool;
    fn scan(&self, paths: &[PathBuf]) -> Vec<PathBuf>;
    fn parse(&self, path: &PathBuf) -> Result<Conversation, ParseError>;
}
