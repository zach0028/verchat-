use std::time::Instant;

use crate::store::Store;
use crate::store::sqlite::{ConversationSummary, SearchResult};
use crate::model::{Conversation, Role};

/// L'écran actif de la TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    Search,
    ConversationView,
    Sources,
    Stats,
}

/// État global de l'application TUI.
pub struct App {
    /// Store SQLite.
    pub store: Store,

    /// Écran actif.
    pub screen: Screen,

    /// L'application est-elle en cours d'exécution ?
    pub running: bool,

    // ── Dashboard ──
    /// Liste des conversations affichées.
    pub conversations: Vec<ConversationSummary>,
    /// Index de la conversation sélectionnée.
    pub selected_index: usize,

    // ── Search ──
    /// Texte saisi dans la barre de recherche.
    pub search_query: String,
    /// Résultats de recherche.
    pub search_results: Vec<SearchResult>,
    /// Index du résultat sélectionné.
    pub search_selected: usize,
    /// La barre de recherche est-elle active ?
    pub search_focused: bool,

    // ── Conversation view ──
    /// Conversation actuellement affichée.
    pub current_conversation: Option<Conversation>,
    /// Position de scroll dans la conversation.
    pub scroll_offset: u16,

    // ── Launch overlay ──
    /// L'overlay de lancement est-il visible ?
    pub launch_visible: bool,
    /// Index sélectionné dans le menu de lancement.
    pub launch_selected: usize,
    /// Étape du launch : 0 = choix outil, 1 = fenêtre contexte, 2 = confirmation
    pub launch_step: u8,
    /// Saisie du nombre de tokens max.
    pub launch_token_input: String,
    /// Analyse de la conversation pour le launch.
    pub launch_analysis: Option<crate::export::compress::ConversationAnalysis>,

    // ── Notifications ──
    /// Message de notification éphémère (s'efface après quelques secondes).
    pub notification: Option<(String, Instant)>,
}

impl App {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            screen: Screen::Dashboard,
            running: true,
            conversations: Vec::new(),
            selected_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_focused: false,
            current_conversation: None,
            scroll_offset: 0,
            launch_visible: false,
            launch_selected: 0,
            launch_step: 0,
            launch_token_input: String::new(),
            launch_analysis: None,
            notification: None,
        }
    }

    /// Charge les conversations récentes depuis le store.
    pub fn load_conversations(&mut self) {
        if let Ok(convs) = self.store.list(1000, 0) {
            self.conversations = convs;
            if self.selected_index >= self.conversations.len() {
                self.selected_index = 0;
            }
        }
    }

    /// Lance une recherche full-text.
    pub fn perform_search(&mut self) {
        if self.search_query.trim().is_empty() {
            self.search_results.clear();
            return;
        }
        if let Ok(results) = self.store.search(&self.search_query, 50) {
            self.search_results = results;
            self.search_selected = 0;
        }
    }

    /// Ouvre la conversation sélectionnée.
    pub fn open_selected_conversation(&mut self) {
        let id = match self.screen {
            Screen::Dashboard => {
                self.conversations.get(self.selected_index).map(|c| c.id)
            }
            Screen::Search => {
                self.search_results.get(self.search_selected).map(|r| r.conversation.id)
            }
            _ => None,
        };

        if let Some(id) = id {
            let prefix = &id.to_string()[..8];
            if let Ok(Some(conv)) = self.store.get_by_id_prefix(prefix) {
                self.current_conversation = Some(conv);
                self.scroll_offset = 0;
                self.screen = Screen::ConversationView;
            }
        }
    }

    /// Navigation : élément précédent.
    pub fn select_previous(&mut self) {
        match self.screen {
            Screen::Dashboard => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            Screen::Search => {
                if self.search_selected > 0 {
                    self.search_selected -= 1;
                }
            }
            Screen::ConversationView => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
            }
            _ => {}
        }
    }

    /// Navigation : élément suivant.
    pub fn select_next(&mut self) {
        match self.screen {
            Screen::Dashboard => {
                if self.selected_index + 1 < self.conversations.len() {
                    self.selected_index += 1;
                }
            }
            Screen::Search => {
                if self.search_selected + 1 < self.search_results.len() {
                    self.search_selected += 1;
                }
            }
            Screen::ConversationView => {
                self.scroll_offset = self.scroll_offset.saturating_add(3);
            }
            _ => {}
        }
    }

    /// Retour à l'écran précédent.
    pub fn go_back(&mut self) {
        match self.screen {
            Screen::Search if self.search_focused => {
                self.search_focused = false;
            }
            Screen::Search => {
                self.screen = Screen::Dashboard;
                self.search_query.clear();
                self.search_results.clear();
            }
            Screen::ConversationView => {
                self.screen = if self.search_results.is_empty() {
                    Screen::Dashboard
                } else {
                    Screen::Search
                };
                self.current_conversation = None;
            }
            Screen::Sources | Screen::Stats => {
                self.screen = Screen::Dashboard;
            }
            Screen::Dashboard => {
                self.running = false;
            }
        }
    }

    /// Active le mode recherche.
    pub fn enter_search(&mut self) {
        self.screen = Screen::Search;
        self.search_focused = true;
    }

    /// Affiche une notification éphémère (disparaît après 2s).
    pub fn notify(&mut self, message: String) {
        self.notification = Some((message, Instant::now()));
    }

    /// Vérifie si la notification a expiré.
    pub fn tick_notification(&mut self) {
        if let Some((_, created)) = &self.notification {
            if created.elapsed().as_secs() >= 2 {
                self.notification = None;
            }
        }
    }

    /// Copie la conversation actuelle dans le clipboard au format Markdown.
    pub fn copy_conversation_to_clipboard(&mut self) {
        let conv = match &self.current_conversation {
            Some(c) => c,
            None => return,
        };

        let markdown = format_conversation_as_markdown(conv);

        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&markdown)) {
            Ok(_) => {
                let msg_count = conv.messages.len();
                self.notify(format!("✓ Conversation copiée ({msg_count} messages)"));
            }
            Err(e) => {
                self.notify(format!("✗ Erreur clipboard: {e}"));
            }
        }
    }

    /// Ouvre l'overlay de lancement (étape 0 : choix outil).
    pub fn open_launch_menu(&mut self) {
        if self.current_conversation.is_some() {
            self.launch_visible = true;
            self.launch_selected = 0;
            self.launch_step = 0;
            self.launch_token_input.clear();
            self.launch_analysis = None;
        }
    }

    /// Ferme l'overlay de lancement.
    pub fn close_launch_menu(&mut self) {
        self.launch_visible = false;
        self.launch_step = 0;
    }

    /// Passe à l'étape suivante du launch.
    pub fn launch_confirm_step(&mut self) {
        match self.launch_step {
            0 => {
                // Étape 0 → 1 : outil choisi, passer à la fenêtre de contexte
                if let Some(conv) = &self.current_conversation {
                    self.launch_analysis = Some(crate::export::compress::analyze(conv));
                }
                self.launch_step = 1;
                self.launch_token_input = "128000".to_string();
            }
            1 => {
                // Étape 1 → exécuter le launch avec compression
                self.execute_launch();
            }
            _ => {}
        }
    }

    /// Exécute le lancement avec compression si nécessaire.
    fn execute_launch(&mut self) {
        let targets = crate::export::available_targets();
        let target = match targets.get(self.launch_selected) {
            Some(t) => t.clone(),
            None => return,
        };

        let max_tokens: usize = self.launch_token_input
            .parse()
            .unwrap_or(128_000);

        let conv = match &self.current_conversation {
            Some(c) => c,
            None => return,
        };

        // Comprimer la conversation
        let compressed = crate::export::compress::compress(conv, max_tokens);

        // Créer une conversation synthétique avec le message compressé
        let launch_conv = Conversation::new(
            conv.title.clone(),
            conv.source,
            conv.model.clone(),
            conv.source_path.clone(),
            conv.created_at,
            conv.updated_at,
            vec![crate::model::Message::new(
                Role::User,
                compressed,
                Some(chrono::Utc::now()),
            )],
        );

        let result = crate::export::launch(&launch_conv, &target);
        self.launch_visible = false;
        self.launch_step = 0;
        self.notify(result);
    }

    /// Applique un preset de tokens.
    pub fn launch_set_preset(&mut self, tokens: &str) {
        self.launch_token_input = tokens.to_string();
    }

    /// Navigation dans le menu de lancement.
    pub fn launch_select_prev(&mut self) {
        if self.launch_selected > 0 {
            self.launch_selected -= 1;
        }
    }

    pub fn launch_select_next(&mut self) {
        let count = crate::export::available_targets().len();
        if self.launch_selected + 1 < count {
            self.launch_selected += 1;
        }
    }
}

/// Formate une conversation en Markdown propre, prêt à coller dans un autre outil.
fn format_conversation_as_markdown(conv: &Conversation) -> String {
    let mut md = String::new();

    // En-tête
    md.push_str(&format!("# {}\n\n", conv.title));
    md.push_str(&format!(
        "> Source: {} | Model: {} | Date: {}\n\n",
        conv.source,
        conv.model.as_deref().unwrap_or("unknown"),
        conv.created_at.format("%Y-%m-%d %H:%M"),
    ));
    md.push_str("---\n\n");

    // Messages
    for msg in &conv.messages {
        let role_label = match msg.role {
            Role::User => "**User**",
            Role::Assistant => "**Assistant**",
            Role::System => "**System**",
        };

        let ts = msg
            .timestamp
            .map(|t| format!(" _{}_", t.format("%H:%M")))
            .unwrap_or_default();

        md.push_str(&format!("### {role_label}{ts}\n\n"));
        md.push_str(&msg.content);
        md.push_str("\n\n---\n\n");
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        let store = Store::open_in_memory().unwrap();
        App::new(store)
    }

    #[test]
    fn test_initial_state() {
        let app = make_app();
        assert_eq!(app.screen, Screen::Dashboard);
        assert!(app.running);
        assert!(app.search_query.is_empty());
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_enter_search() {
        let mut app = make_app();
        app.enter_search();
        assert_eq!(app.screen, Screen::Search);
        assert!(app.search_focused);
    }

    #[test]
    fn test_go_back_from_search() {
        let mut app = make_app();
        app.enter_search();
        app.search_query = "test".to_string();
        app.go_back(); // unfocus search bar
        assert!(!app.search_focused);
        app.go_back(); // back to dashboard
        assert_eq!(app.screen, Screen::Dashboard);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_go_back_from_dashboard_quits() {
        let mut app = make_app();
        assert!(app.running);
        app.go_back();
        assert!(!app.running);
    }

    #[test]
    fn test_navigation_empty_list() {
        let mut app = make_app();
        // Should not panic on empty list
        app.select_next();
        app.select_previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_navigation_bounds() {
        let mut app = make_app();

        // Insert test conversations
        use crate::model::{Conversation, Message, Role, Source};
        use chrono::Utc;
        for i in 0..5 {
            let conv = Conversation::new(
                format!("Conv {i}"),
                Source::ClaudeCode,
                None,
                format!("/tmp/test_{i}.jsonl"),
                Utc::now(),
                Utc::now(),
                vec![Message::new(Role::User, "test".to_string(), None)],
            );
            app.store.insert(&conv).unwrap();
        }
        app.load_conversations();

        assert_eq!(app.selected_index, 0);
        app.select_previous(); // should stay at 0
        assert_eq!(app.selected_index, 0);

        for _ in 0..10 {
            app.select_next(); // should cap at 4
        }
        assert_eq!(app.selected_index, 4);
    }

    #[test]
    fn test_search_and_results() {
        let mut app = make_app();

        use crate::model::{Conversation, Message, Role, Source};
        use chrono::Utc;
        let conv = Conversation::new(
            "Auth middleware fix".to_string(),
            Source::ClaudeCode,
            Some("claude-opus-4-6".to_string()),
            "/tmp/auth.jsonl".to_string(),
            Utc::now(),
            Utc::now(),
            vec![
                Message::new(Role::User, "Fix the auth middleware".to_string(), None),
                Message::new(Role::Assistant, "Found the bug in JWT validation".to_string(), None),
            ],
        );
        app.store.insert(&conv).unwrap();

        app.enter_search();
        app.search_query = "auth".to_string();
        app.perform_search();
        assert_eq!(app.search_results.len(), 1);

        app.search_query = "nonexistent".to_string();
        app.perform_search();
        assert!(app.search_results.is_empty());
    }
}
