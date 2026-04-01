mod commands;

use clap::{Parser, Subcommand};

/// VER.CHAT — Le Git des conversations IA
#[derive(Parser)]
#[command(name = "verchat", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialiser VER.CHAT : détection des sources et création de la config
    Init,

    /// Importer les conversations depuis les outils IA détectés
    Import {
        /// Importer uniquement depuis une source spécifique
        #[arg(value_name = "SOURCE")]
        source: Option<String>,

        /// Importer depuis toutes les sources détectées
        #[arg(long, default_value_t = false)]
        auto: bool,
    },

    /// Rechercher dans toutes les conversations
    Search {
        /// Le terme à rechercher
        query: String,

        /// Filtrer par source (claude-code, lm-studio, continue-dev, aider)
        #[arg(long)]
        source: Option<String>,

        /// Nombre max de résultats
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },

    /// Lister les conversations récentes
    Log {
        /// Nombre de conversations à afficher
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,

        /// Filtrer par source
        #[arg(long)]
        source: Option<String>,
    },

    /// Afficher le contenu d'une conversation
    Show {
        /// ID ou début de l'ID de la conversation
        id: String,
    },

    /// Copier une conversation dans le clipboard (format Markdown)
    Copy {
        /// ID ou début de l'ID de la conversation
        id: String,
    },

    /// Gérer les sources de conversations
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },

    /// Afficher l'état du store
    Status,
}

#[derive(Subcommand)]
pub enum SourceAction {
    /// Lister les sources configurées
    List,

    /// Ajouter un chemin à une source
    Add {
        /// Nom de la source (claude-code, lm-studio, continue-dev, aider)
        source: String,
        /// Chemin à ajouter
        path: String,
    },

    /// Retirer un chemin d'une source
    Remove {
        /// Nom de la source
        source: String,
        /// Chemin à retirer
        path: String,
    },
}

pub use commands::run;
