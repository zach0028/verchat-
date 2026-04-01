use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration globale de VER.CHAT, persistée en TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Sources de conversations configurées.
    #[serde(default)]
    pub sources: HashMap<String, SourceConfig>,
}

/// Configuration d'une source de conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// La source est-elle active ?
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Chemins à scanner pour cette source.
    #[serde(default)]
    pub paths: Vec<String>,

    /// Chemins à exclure.
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Chemin par défaut du fichier de config.
    pub fn default_path() -> PathBuf {
        verchat_dir().join("config.toml")
    }

    /// Charge la config depuis le fichier. Retourne une config par défaut si le fichier n'existe pas.
    pub fn load() -> Self {
        let path = Self::default_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => eprintln!("Warning: invalid config.toml: {e}"),
                },
                Err(e) => eprintln!("Warning: could not read config.toml: {e}"),
            }
        }
        Self::default()
    }

    /// Sauvegarde la config dans le fichier.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::default_path();
        fs::create_dir_all(path.parent().unwrap())?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&path, content)
    }

    /// Génère une config par défaut en détectant les outils installés.
    pub fn detect() -> Self {
        let home = home_dir();
        let mut sources = HashMap::new();

        // Claude Code
        let claude_path = home.join(".claude").join("projects");
        sources.insert(
            "claude-code".to_string(),
            SourceConfig {
                enabled: claude_path.is_dir(),
                paths: vec![claude_path.to_string_lossy().to_string()],
                exclude: Vec::new(),
            },
        );

        // LM Studio
        let lm_path = home.join(".lmstudio").join("conversations");
        sources.insert(
            "lm-studio".to_string(),
            SourceConfig {
                enabled: lm_path.is_dir(),
                paths: vec![lm_path.to_string_lossy().to_string()],
                exclude: Vec::new(),
            },
        );

        // Continue.dev
        let continue_path = home.join(".continue").join("sessions");
        sources.insert(
            "continue-dev".to_string(),
            SourceConfig {
                enabled: continue_path.is_dir(),
                paths: vec![continue_path.to_string_lossy().to_string()],
                exclude: Vec::new(),
            },
        );

        // Aider
        let aider_global = home.join(".aider.chat.history.md");
        sources.insert(
            "aider".to_string(),
            SourceConfig {
                enabled: aider_global.exists(),
                paths: if aider_global.exists() {
                    vec![aider_global.to_string_lossy().to_string()]
                } else {
                    Vec::new()
                },
                exclude: Vec::new(),
            },
        );

        Self { sources }
    }

    /// Retourne les chemins configurés pour une source donnée.
    pub fn paths_for(&self, source_key: &str) -> Vec<PathBuf> {
        self.sources
            .get(source_key)
            .filter(|s| s.enabled)
            .map(|s| s.paths.iter().map(PathBuf::from).collect())
            .unwrap_or_default()
    }

    /// Ajoute un chemin à une source.
    pub fn add_path(&mut self, source_key: &str, path: &str) {
        let entry = self.sources.entry(source_key.to_string()).or_insert(SourceConfig {
            enabled: true,
            paths: Vec::new(),
            exclude: Vec::new(),
        });
        let p = path.to_string();
        if !entry.paths.contains(&p) {
            entry.paths.push(p);
        }
    }

    /// Retire un chemin d'une source.
    pub fn remove_path(&mut self, source_key: &str, path: &str) {
        if let Some(source) = self.sources.get_mut(source_key) {
            source.paths.retain(|p| p != path);
        }
    }

    /// Active ou désactive une source.
    pub fn set_enabled(&mut self, source_key: &str, enabled: bool) {
        if let Some(source) = self.sources.get_mut(source_key) {
            source.enabled = enabled;
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::detect()
    }
}

/// Répertoire VER.CHAT (~/.verchat/).
pub fn verchat_dir() -> PathBuf {
    home_dir().join(".verchat")
}

/// Chemin de la base de données.
pub fn db_path() -> PathBuf {
    verchat_dir().join("store.db")
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}
