pub mod compress;
mod lm_studio;
mod continue_dev;

use std::process::Command;

use crate::model::{Conversation, Role, Source};

/// Cible de lancement disponible.
#[derive(Debug, Clone)]
pub struct LaunchTarget {
    /// Nom affiché dans le menu.
    pub name: &'static str,
    /// Clé de la source.
    pub source: Source,
    /// Méthode d'injection.
    pub method: LaunchMethod,
}

#[derive(Debug, Clone)]
pub enum LaunchMethod {
    /// Injection native — on écrit un fichier dans le dossier de l'outil puis on ouvre l'app.
    NativeInject,
    /// Copie dans le clipboard puis on ouvre l'app.
    Clipboard,
}

/// Retourne la liste des cibles de lancement disponibles.
pub fn available_targets() -> Vec<LaunchTarget> {
    vec![
        LaunchTarget {
            name: "LM Studio",
            source: Source::LmStudio,
            method: LaunchMethod::NativeInject,
        },
        LaunchTarget {
            name: "Continue.dev (VS Code)",
            source: Source::ContinueDev,
            method: LaunchMethod::NativeInject,
        },
        LaunchTarget {
            name: "Claude Code",
            source: Source::ClaudeCode,
            method: LaunchMethod::Clipboard,
        },
        LaunchTarget {
            name: "Aider",
            source: Source::Aider,
            method: LaunchMethod::Clipboard,
        },
        LaunchTarget {
            name: "Clipboard uniquement",
            source: Source::ClaudeCode,
            method: LaunchMethod::Clipboard,
        },
    ]
}

/// Lance une conversation vers la cible choisie.
/// Injecte le fichier (si possible), copie dans le clipboard, et ouvre l'application.
pub fn launch(conv: &Conversation, target: &LaunchTarget) -> String {
    // Étape 1 : injection ou clipboard
    let inject_msg = match target.method {
        LaunchMethod::NativeInject => match target.source {
            Source::LmStudio => match lm_studio::inject(conv) {
                Ok(path) => format!("✓ Injecté dans LM Studio ({})", path.display()),
                Err(e) => return format!("✗ Erreur LM Studio: {e}"),
            },
            Source::ContinueDev => match continue_dev::inject(conv) {
                Ok(path) => format!("✓ Injecté dans Continue.dev ({})", path.display()),
                Err(e) => return format!("✗ Erreur Continue.dev: {e}"),
            },
            _ => copy_to_clipboard_msg(conv),
        },
        LaunchMethod::Clipboard => copy_to_clipboard_msg(conv),
    };

    // Étape 2 : ouvrir l'application (sauf clipboard uniquement)
    if target.name == "Clipboard uniquement" {
        return inject_msg;
    }

    let open_result = open_app(&target.source, target.name);

    match open_result {
        Some(ok_msg) => format!("{inject_msg} — {ok_msg}"),
        None => inject_msg,
    }
}

/// Ouvre l'application associée à une source.
/// Retourne un message de succès, ou None si on ne peut pas l'ouvrir.
fn open_app(source: &Source, name: &str) -> Option<String> {
    let result = match source {
        Source::LmStudio => {
            // macOS : ouvrir LM Studio via `open`
            Command::new("open")
                .arg("-a")
                .arg("LM Studio")
                .spawn()
        }
        Source::ContinueDev => {
            // Ouvrir VS Code (Continue.dev est une extension)
            Command::new("code")
                .spawn()
        }
        Source::ClaudeCode => {
            // Claude Code est un CLI — ouvrir un nouveau terminal avec claude
            // On utilise `open -a Terminal` sur macOS comme fallback
            Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .spawn()
        }
        Source::Aider => {
            Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .spawn()
        }
        Source::GeminiCli | Source::OpenCode => {
            Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .spawn()
        }
        Source::Cursor => {
            Command::new("open")
                .arg("-a")
                .arg("Cursor")
                .spawn()
        }
        Source::Windsurf => {
            Command::new("open")
                .arg("-a")
                .arg("Windsurf")
                .spawn()
        }
    };

    match result {
        Ok(_) => Some(format!("{name} ouvert")),
        Err(_) => None,
    }
}

fn copy_to_clipboard_msg(conv: &Conversation) -> String {
    let md = format_as_markdown(conv);
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&md)) {
        Ok(_) => format!("✓ Copié dans le clipboard ({} messages)", conv.messages.len()),
        Err(e) => format!("✗ Erreur clipboard: {e}"),
    }
}

fn format_as_markdown(conv: &Conversation) -> String {
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
    md
}
