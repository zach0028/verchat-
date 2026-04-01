use crate::config::{self, Config};
use crate::parser::Parser;
use crate::parser::claude_code::ClaudeCodeParser;
use crate::parser::lm_studio::LmStudioParser;
use crate::parser::continue_dev::ContinueDevParser;
use crate::parser::aider::AiderParser;
use crate::store::Store;
use super::{Commands, SourceAction};

/// Ouvre le store (le crée si nécessaire).
fn open_store() -> Store {
    let db_path = config::db_path();
    std::fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create ~/.verchat/");
    Store::open(&db_path).expect("Failed to open database")
}

/// Retourne tous les parsers disponibles.
fn all_parsers() -> Vec<(&'static str, Box<dyn Parser>)> {
    vec![
        ("claude-code", Box::new(ClaudeCodeParser)),
        ("lm-studio", Box::new(LmStudioParser)),
        ("continue-dev", Box::new(ContinueDevParser)),
        ("aider", Box::new(AiderParser)),
    ]
}

/// Dispatch la commande.
pub fn run(command: Commands) {
    match command {
        Commands::Init => cmd_init(),
        Commands::Import { source, auto } => cmd_import(source, auto),
        Commands::Search { query, source, limit } => cmd_search(&query, source.as_deref(), limit),
        Commands::Log { limit, source } => cmd_log(limit, source.as_deref()),
        Commands::Show { id } => cmd_show(&id),
        Commands::Copy { id } => cmd_copy(&id),
        Commands::Source { action } => cmd_source(action),
        Commands::Status => cmd_status(),
    }
}

// ── Init ────────────────────────────────────────────────────────────

fn cmd_init() {
    let config_path = Config::default_path();

    if config_path.exists() {
        println!("VER.CHAT est déjà initialisé.");
        println!("  Config: {}", config_path.display());
        println!("  Utilisez `verchat source list` pour voir les sources.");
        return;
    }

    println!("Initialisation de VER.CHAT...\n");
    println!("Détection des outils IA installés...\n");

    let config = Config::detect();

    for (key, source) in &config.sources {
        let status = if source.enabled { "✓" } else { "○" };
        let detail = if source.enabled {
            format!("{}", source.paths.join(", "))
        } else {
            "non détecté".to_string()
        };
        println!("  {status} {key}");
        println!("    {detail}\n");
    }

    match config.save() {
        Ok(_) => {
            println!("Configuration sauvegardée: {}", config_path.display());
            println!("\nProchaine étape: `verchat import --auto`");
        }
        Err(e) => eprintln!("Erreur: {e}"),
    }
}

// ── Import ──────────────────────────────────────────────────────────

fn cmd_import(source_filter: Option<String>, _auto: bool) {
    let config = Config::load();
    let store = open_store();
    let parsers = all_parsers();

    for (key, p) in &parsers {
        // Filtre par source si spécifié
        if let Some(ref filter) = source_filter {
            if !key.contains(&filter.to_lowercase()) {
                continue;
            }
        }

        // Vérifier si activé dans la config
        let source_config = config.sources.get(*key);
        if source_config.is_some_and(|s| !s.enabled) {
            println!("  ○ {} — désactivé", p.name());
            continue;
        }

        if !p.detect() {
            println!("  ○ {} — non détecté", p.name());
            continue;
        }

        let paths = config.paths_for(key);
        let files = p.scan(&paths);
        println!("  ● {} — {} files found", p.name(), files.len());

        let mut imported = 0;
        let mut skipped = 0;
        let mut errors = 0;

        for path in &files {
            match p.parse(path) {
                Ok(conv) => match store.insert(&conv) {
                    Ok(true) => imported += 1,
                    Ok(false) => skipped += 1,
                    Err(e) => {
                        eprintln!("    error: {e}");
                        errors += 1;
                    }
                },
                Err(_) => errors += 1,
            }
        }

        println!("    imported: {imported} | skipped: {skipped} | errors: {errors}");
    }

    let total = store.count().unwrap_or(0);
    println!("\n  Total: {total} conversations in store");
}

// ── Search ──────────────────────────────────────────────────────────

fn cmd_search(query: &str, _source_filter: Option<&str>, limit: usize) {
    let store = open_store();

    match store.search(query, limit) {
        Ok(results) if results.is_empty() => {
            println!("No results for \"{query}\"");
        }
        Ok(results) => {
            println!("{} result{} for \"{query}\"\n",
                results.len(),
                if results.len() > 1 { "s" } else { "" },
            );
            for r in &results {
                let c = &r.conversation;
                println!("  {} │ {} │ {}m",
                    c.source,
                    truncate(&c.title, 50),
                    c.message_count,
                );
                let snippet: String = r.snippet
                    .chars()
                    .take(120)
                    .map(|c| if c == '\n' { ' ' } else { c })
                    .collect();
                println!("    {snippet}\n");
            }
        }
        Err(e) => eprintln!("Search error: {e}"),
    }
}

// ── Log ─────────────────────────────────────────────────────────────

fn cmd_log(limit: usize, _source_filter: Option<&str>) {
    let store = open_store();

    match store.list(limit, 0) {
        Ok(conversations) if conversations.is_empty() => {
            println!("No conversations yet. Run `verchat import --auto` first.");
        }
        Ok(conversations) => {
            for conv in &conversations {
                let date = conv.updated_at.format("%Y-%m-%d %H:%M");
                let fav = if conv.favorite { "★ " } else { "  " };
                let short_id = &conv.id.to_string()[..8];
                let title = truncate(&conv.title, 50);
                println!("{fav}{short_id} │ {date} │ {:12} │ {:>3}m │ {title}",
                    conv.source.to_string(),
                    conv.message_count,
                );
            }
            println!("\n{} conversations", conversations.len());
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

// ── Show ────────────────────────────────────────────────────────────

fn cmd_show(id_prefix: &str) {
    let store = open_store();

    match store.get_by_id_prefix(id_prefix) {
        Ok(Some(conv)) => {
            println!("── {} ──", conv.title);
            println!("   {} │ {} │ {} │ {}m\n",
                conv.source,
                conv.model.as_deref().unwrap_or("unknown"),
                conv.created_at.format("%Y-%m-%d %H:%M"),
                conv.messages.len(),
            );
            for msg in &conv.messages {
                let ts = msg.timestamp
                    .map(|t| t.format("%H:%M").to_string())
                    .unwrap_or_default();
                println!("  ┃ {} · {ts}", msg.role);
                for line in msg.content.lines().take(20) {
                    println!("  ┃ {line}");
                }
                if msg.content.lines().count() > 20 {
                    println!("  ┃ ... ({} more lines)", msg.content.lines().count() - 20);
                }
                println!();
            }
        }
        Ok(None) => eprintln!("No conversation found matching \"{id_prefix}\""),
        Err(e) => eprintln!("Error: {e}"),
    }
}

// ── Copy ────────────────────────────────────────────────────────────

fn cmd_copy(id_prefix: &str) {
    let store = open_store();

    match store.get_by_id_prefix(id_prefix) {
        Ok(Some(conv)) => {
            use crate::model::Role;
            let mut md = format!("# {}\n\n", conv.title);
            md.push_str(&format!(
                "> Source: {} | Model: {} | Date: {}\n\n---\n\n",
                conv.source,
                conv.model.as_deref().unwrap_or("unknown"),
                conv.created_at.format("%Y-%m-%d %H:%M"),
            ));
            for msg in &conv.messages {
                let label = match msg.role {
                    Role::User => "**User**",
                    Role::Assistant => "**Assistant**",
                    Role::System => "**System**",
                };
                md.push_str(&format!("### {label}\n\n{}\n\n---\n\n", msg.content));
            }

            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&md)) {
                Ok(_) => println!("✓ Conversation copiée dans le clipboard ({} messages)", conv.messages.len()),
                Err(e) => eprintln!("✗ Erreur clipboard: {e}"),
            }
        }
        Ok(None) => eprintln!("No conversation found matching \"{id_prefix}\""),
        Err(e) => eprintln!("Error: {e}"),
    }
}

// ── Source ───────────────────────────────────────────────────────────

fn cmd_source(action: SourceAction) {
    match action {
        SourceAction::List => {
            let config = Config::load();
            println!("Sources configurées:\n");
            for (key, source) in &config.sources {
                let status = if source.enabled { "●" } else { "○" };
                println!("  {status} {key} {}", if source.enabled { "(actif)" } else { "(désactivé)" });
                for path in &source.paths {
                    println!("    → {path}");
                }
                if !source.exclude.is_empty() {
                    for ex in &source.exclude {
                        println!("    ✗ {ex}");
                    }
                }
                println!();
            }
        }
        SourceAction::Add { source, path } => {
            let mut config = Config::load();
            let abs_path = resolve_path(&path);
            config.add_path(&source, &abs_path);
            match config.save() {
                Ok(_) => println!("✓ Chemin ajouté à {source}: {abs_path}"),
                Err(e) => eprintln!("✗ Erreur: {e}"),
            }
        }
        SourceAction::Remove { source, path } => {
            let mut config = Config::load();
            config.remove_path(&source, &path);
            match config.save() {
                Ok(_) => println!("✓ Chemin retiré de {source}: {path}"),
                Err(e) => eprintln!("✗ Erreur: {e}"),
            }
        }
    }
}

// ── Status ──────────────────────────────────────────────────────────

fn cmd_status() {
    let store = open_store();
    let config = Config::load();
    let total = store.count().unwrap_or(0);
    let db_path = config::db_path();
    let db_size = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    println!("VER.CHAT v{}\n", env!("CARGO_PKG_VERSION"));
    println!("  Database: {}", db_path.display());
    println!("  Config:   {}", Config::default_path().display());
    println!("  Size:     {}", format_bytes(db_size));
    println!("  Total:    {total} conversations\n");

    println!("Sources:");
    let parsers = all_parsers();
    for (key, p) in &parsers {
        let enabled = config.sources.get(*key).is_none_or(|s| s.enabled);
        if !enabled {
            println!("  ○ {} — désactivé", p.name());
            continue;
        }
        if p.detect() {
            let paths = config.paths_for(key);
            let files = p.scan(&paths);
            println!("  ● {} — {} files detected", p.name(), files.len());
        } else {
            println!("  ○ {} — non détecté", p.name());
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let end = max.saturating_sub(3);
        format!("{}...", chars[..end].iter().collect::<String>())
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn resolve_path(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    // Try to canonicalize, fallback to as-is
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}
