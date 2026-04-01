use std::path::PathBuf;
use std::sync::mpsc;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;

/// Événements envoyés par le file watcher vers la TUI.
#[derive(Debug)]
pub enum WatchEvent {
    /// Un fichier de conversation a été modifié ou créé.
    FileChanged(PathBuf),
}

/// Démarre le file watcher en arrière-plan.
/// Retourne un receiver pour consommer les événements depuis la boucle TUI.
pub fn start(config: &Config) -> Option<mpsc::Receiver<WatchEvent>> {
    let (tx, rx) = mpsc::channel();

    // Collecter les chemins à surveiller
    let mut watch_paths: Vec<PathBuf> = Vec::new();
    for source in config.sources.values() {
        if source.enabled {
            for path in &source.paths {
                let p = PathBuf::from(path);
                if p.exists() {
                    watch_paths.push(p);
                }
            }
        }
    }

    if watch_paths.is_empty() {
        return Some(rx);
    }

    let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    // Créer le watcher
    let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            let _ = notify_tx.send(event);
        }
    }) {
        Ok(w) => w,
        Err(_) => return Some(rx),
    };

    // Surveiller chaque chemin
    for path in &watch_paths {
        let _ = watcher.watch(path, RecursiveMode::Recursive);
    }

    // Thread qui filtre les événements et les envoie à la TUI
    std::thread::spawn(move || {
        let _watcher = watcher; // Garder le watcher vivant

        while let Ok(event) = notify_rx.recv() {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    for path in event.paths {
                        // Ne notifier que pour les fichiers pertinents
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                        let relevant = ext == "jsonl"
                            || name.ends_with(".conversation.json")
                            || (ext == "json" && !name.contains("sessions.json"))
                            || name == ".aider.chat.history.md";

                        if relevant {
                            let _ = tx.send(WatchEvent::FileChanged(path));
                        }
                    }
                }
                _ => {}
            }
        }
    });

    Some(rx)
}
