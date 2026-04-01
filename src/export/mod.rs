pub mod compress;
mod continue_dev;
mod lm_studio;
mod opencode;

use std::process::{Command, Stdio};

use crate::model::{Conversation, Role, Source};

/// Cible de lancement disponible.
#[derive(Debug, Clone)]
pub struct LaunchTarget {
    pub name: &'static str,
    pub source: Source,
    pub method: LaunchMethod,
}

#[derive(Debug, Clone)]
pub enum LaunchMethod {
    NativeInject,
    Clipboard,
}

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
            name: "OpenCode",
            source: Source::OpenCode,
            method: LaunchMethod::NativeInject,
        },
        LaunchTarget {
            name: "Claude Code",
            source: Source::ClaudeCode,
            method: LaunchMethod::Clipboard,
        },
        LaunchTarget {
            name: "Gemini CLI",
            source: Source::GeminiCli,
            method: LaunchMethod::Clipboard,
        },
        LaunchTarget {
            name: "Cursor",
            source: Source::Cursor,
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
pub fn launch(conv: &Conversation, target: &LaunchTarget) -> String {
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
            Source::OpenCode => match opencode::inject(conv) {
                Ok(session_id) => format!("✓ Injecté dans OpenCode ({session_id})"),
                Err(e) => return format!("✗ Erreur OpenCode: {e}"),
            },
            _ => clipboard_copy_sync(conv),
        },
        LaunchMethod::Clipboard => clipboard_copy_sync(conv),
    };

    if target.name == "Clipboard uniquement" {
        return inject_msg;
    }

    let name = target.name.to_string();
    let source = target.source;
    std::thread::spawn(move || {
        open_app_blocking(&source);
    });

    format!("{inject_msg} — {name} ouvert")
}

fn clipboard_copy_sync(conv: &Conversation) -> String {
    let md = format_as_markdown(conv);
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&md)) {
        Ok(_) => format!("✓ Copié ({} messages)", conv.messages.len()),
        Err(e) => format!("✗ Clipboard: {e}"),
    }
}

pub fn clipboard_copy_async(text: String) -> String {
    let len_hint = text.len();
    std::thread::spawn(move || {
        let _ = arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text));
    });
    format!("✓ Copié (~{} chars)", len_hint)
}

fn open_app_blocking(source: &Source) {
    let (cmd, args): (&str, Vec<&str>) = match source {
        Source::LmStudio => ("open", vec!["-a", "LM Studio"]),
        Source::ContinueDev => ("code", vec![]),
        Source::Cursor => ("open", vec!["-a", "Cursor"]),
        Source::Windsurf => ("open", vec!["-a", "Windsurf"]),
        Source::OpenCode => return, // CLI tool, pas d'app à ouvrir
        _ => ("open", vec!["-a", "Terminal"]),
    };

    if let Ok(mut child) = Command::new(cmd)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        let _ = child.wait();
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
