pub mod app;
pub mod screens;
pub mod theme;
pub mod watcher;

use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::Terminal;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph};

use app::{App, Screen};
use theme::Theme;
use crate::config::Config;
use crate::parser::{Parser, claude_code::ClaudeCodeParser, lm_studio::LmStudioParser, continue_dev::ContinueDevParser, aider::AiderParser};

/// Lance la TUI interactive.
pub fn run(store: crate::store::Store) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(store);
    app.load_conversations();

    // Démarrer le file watcher
    let config = Config::load();
    let watch_rx = watcher::start(&config);

    let result = run_loop(&mut terminal, &mut app, watch_rx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    watch_rx: Option<mpsc::Receiver<watcher::WatchEvent>>,
) -> io::Result<()> {
    let mut last_reimport = Instant::now();

    while app.running {
        app.tick_notification();

        // Check file watcher events (non-blocking)
        if let Some(ref rx) = watch_rx {
            let mut has_changes = false;
            while rx.try_recv().is_ok() {
                has_changes = true;
            }
            // Debounce : ré-importer max toutes les 3 secondes
            if has_changes && last_reimport.elapsed() > Duration::from_secs(3) {
                reimport_all(app);
                app.load_conversations();
                last_reimport = Instant::now();
            }
        }

        terminal.draw(|f| render(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.running = false;
                    continue;
                }
                handle_key(app, key.code);
            }
        }
    }
    Ok(())
}

/// Ré-importe toutes les conversations (appelé par le file watcher).
fn reimport_all(app: &mut App) {
    let config = Config::load();
    let parsers: Vec<(&str, Box<dyn Parser>)> = vec![
        ("claude-code", Box::new(ClaudeCodeParser)),
        ("lm-studio", Box::new(LmStudioParser)),
        ("continue-dev", Box::new(ContinueDevParser)),
        ("aider", Box::new(AiderParser)),
    ];

    let mut total_new = 0;
    for (key, p) in &parsers {
        if !p.detect() {
            continue;
        }
        let paths = config.paths_for(key);
        let files = p.scan(&paths);
        for path in &files {
            if let Ok(conv) = p.parse(path) {
                if let Ok(true) = app.store.insert(&conv) {
                    total_new += 1;
                }
            }
        }
    }

    if total_new > 0 {
        app.notify(format!("↻ {total_new} nouvelle(s) conversation(s) détectée(s)"));
    }
}

fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    f.render_widget(
        Block::default().style(Style::default().bg(Theme::base())),
        area,
    );

    // Protéger contre les terminaux très petits
    if area.height < 10 || area.width < 40 {
        f.render_widget(
            Paragraph::new("Terminal trop petit. Redimensionnez la fenêtre.")
                .style(Style::default().fg(Theme::red())),
            area,
        );
        return;
    }

    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(area);

    render_app_header(f, app, layout[0]);

    match app.screen {
        Screen::Dashboard => screens::dashboard::render(f, app, layout[1]),
        Screen::Search => screens::search::render(f, app, layout[1]),
        Screen::ConversationView => screens::conversation::render(f, app, layout[1]),
        Screen::Stats => screens::stats::render(f, app, layout[1]),
        Screen::Sources => {}
    }

    // Launch overlay
    if app.launch_visible {
        render_launch_overlay(f, app, area);
    }

    // Notification overlay
    if let Some((msg, _)) = &app.notification {
        let notif_width = (msg.chars().count() + 4).min(area.width as usize) as u16;
        let notif_area = Rect {
            x: area.width.saturating_sub(notif_width + 2),
            y: area.height.saturating_sub(3),
            width: notif_width,
            height: 3,
        };

        let is_error = msg.starts_with('✗');
        let border_color = if is_error { Theme::red() } else { Theme::green() };
        let bg_color = if is_error { Theme::red() } else { Theme::green() };

        let notif = Paragraph::new(Line::from(Span::styled(
            format!(" {msg} "),
            Style::default().fg(Theme::base()).add_modifier(Modifier::BOLD),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(bg_color)),
        );

        f.render_widget(Clear, notif_area);
        f.render_widget(notif, notif_area);
    }
}

fn render_app_header(f: &mut Frame, _app: &App, area: Rect) {
    let watching = "● watching";

    let title = Line::from(vec![
        Span::styled("  VER", Style::default().fg(Theme::blue()).add_modifier(Modifier::BOLD)),
        Span::styled(".", Style::default().fg(Theme::mauve())),
        Span::styled("CHAT", Style::default().fg(Theme::blue()).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Theme::overlay0()),
        ),
        Span::styled(
            format!(
                "{}{}",
                " ".repeat((area.width as usize).saturating_sub(30)),
                watching,
            ),
            Style::default().fg(Theme::green()),
        ),
    ]);

    let header = Paragraph::new(title).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::surface2()))
            .padding(Padding::new(0, 0, 1, 0)),
    );

    f.render_widget(header, area);
}

fn render_launch_overlay(f: &mut Frame, app: &App, area: Rect) {
    let targets = crate::export::available_targets();

    let overlay_height = (targets.len() as u16 + 4).min(area.height - 4);
    let overlay_width = 45u16.min(area.width - 4);
    let overlay_area = Rect {
        x: (area.width - overlay_width) / 2,
        y: (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    // Fond semi-transparent (clear + block)
    f.render_widget(Clear, overlay_area);

    let mut items: Vec<Line> = Vec::new();
    items.push(Line::from(""));

    for (i, target) in targets.iter().enumerate() {
        let is_selected = i == app.launch_selected;

        let method_label = match target.method {
            crate::export::LaunchMethod::NativeInject => "injecter",
            crate::export::LaunchMethod::Clipboard => "clipboard",
        };

        let source_color = Theme::source_color(&target.source);

        let line = Line::from(vec![
            Span::styled(
                if is_selected { "  ▸ " } else { "    " },
                Style::default().fg(Theme::blue()),
            ),
            Span::styled("● ", Style::default().fg(source_color)),
            Span::styled(
                target.name,
                if is_selected {
                    Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::subtext1())
                },
            ),
            Span::styled(
                format!("  ({method_label})"),
                Style::default().fg(Theme::overlay0()),
            ),
        ]);

        items.push(line);
    }

    items.push(Line::from(""));
    items.push(Line::from(vec![
        Span::styled("  ⏎ ", Theme::status_key()),
        Span::styled("lancer ", Theme::status_label()),
        Span::styled(" esc ", Theme::status_key()),
        Span::styled("annuler", Theme::status_label()),
    ]));

    let block = Block::default()
        .title(Span::styled(" Lancer dans... ", Theme::title_focused()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focused())
        .style(Style::default().bg(Theme::base()));

    let paragraph = Paragraph::new(items).block(block);
    f.render_widget(paragraph, overlay_area);
}

fn handle_key(app: &mut App, key: KeyCode) {
    // Launch overlay actif — intercepter les touches
    if app.launch_visible {
        match key {
            KeyCode::Esc => app.close_launch_menu(),
            KeyCode::Up | KeyCode::Char('k') => app.launch_select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.launch_select_next(),
            KeyCode::Enter => app.execute_launch(),
            _ => {}
        }
        return;
    }

    // Mode recherche : capturer le texte
    if app.screen == Screen::Search && app.search_focused {
        match key {
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.perform_search();
                return;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.perform_search();
                return;
            }
            KeyCode::Esc => {
                app.go_back();
                return;
            }
            KeyCode::Enter | KeyCode::Down => {
                app.search_focused = false;
                return;
            }
            _ => {}
        }
    }

    match key {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('s') if app.screen == Screen::Dashboard => {
            app.screen = Screen::Stats;
        }
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Char('g') if app.screen == Screen::ConversationView => {
            app.scroll_offset = 0;
        }
        KeyCode::Char('G') if app.screen == Screen::ConversationView => {
            app.scroll_offset = u16::MAX;
        }
        KeyCode::Char('c') if app.screen == Screen::ConversationView => {
            app.copy_conversation_to_clipboard();
        }
        KeyCode::Char('l') if app.screen == Screen::ConversationView => {
            app.open_launch_menu();
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.select_previous();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.select_next();
            }
        }
        KeyCode::Enter => app.open_selected_conversation(),
        _ => {}
    }
}
