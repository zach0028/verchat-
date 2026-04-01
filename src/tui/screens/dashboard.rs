use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crate::model::Source;
use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::horizontal([
        Constraint::Length(20), // Sidebar
        Constraint::Min(1),    // Main content
    ])
    .split(area);

    render_sidebar(f, app, layout[0]);
    render_main(f, app, layout[1]);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Min(1),    // Sources
        Constraint::Length(6), // Navigation
    ])
    .split(area);

    // Sources
    let sources: Vec<(Source, &str)> = Source::all()
        .iter()
        .map(|s| (*s, s.display_name()))
        .collect();

    let mut source_items: Vec<ListItem> = Vec::new();
    source_items.push(ListItem::new(Line::from("")));

    for (source, name) in &sources {
        let count = app.conversations.iter().filter(|c| c.source == *source).count();
        if count == 0 {
            continue;
        }
        let color = Theme::source_color(source);
        source_items.push(ListItem::new(Line::from(vec![
            Span::styled(" ● ", Style::default().fg(color)),
            Span::styled(
                *name,
                Style::default().fg(Theme::subtext1()),
            ),
        ])));
        source_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("   {count}"),
                Theme::sidebar_count(),
            ),
        ])));
        source_items.push(ListItem::new(Line::from("")));
    }

    let source_block = Block::default()
        .title(Span::styled(" Sources ", Theme::title_focused()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_sidebar());

    f.render_widget(List::new(source_items).block(source_block), layout[0]);

    // Navigation links
    let nav_items = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" a ", Theme::status_key()),
            Span::styled(" Ajouter", Style::default().fg(Theme::subtext0())),
        ]),
        Line::from(vec![
            Span::styled(" s ", Theme::status_key()),
            Span::styled(" Stats", Style::default().fg(Theme::subtext0())),
        ]),
        Line::from(vec![
            Span::styled(" / ", Theme::status_key()),
            Span::styled(" Search", Style::default().fg(Theme::subtext0())),
        ]),
    ];

    let nav_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_sidebar());

    f.render_widget(Paragraph::new(nav_items).block(nav_block), layout[1]);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(3), // Search bar
        Constraint::Min(1),   // Conversation list
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    render_search_bar(f, layout[0]);
    render_conversation_list(f, app, layout[1]);
    render_status_bar(f, app, layout[2]);
}

fn render_search_bar(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_unfocused())
        .padding(Padding::horizontal(1));

    let content = Line::from(vec![
        Span::styled("  /  ", Style::default().fg(Theme::blue()).add_modifier(Modifier::BOLD)),
        Span::styled("Rechercher...", Theme::placeholder()),
    ]);

    f.render_widget(Paragraph::new(content).block(block), area);
}

fn render_conversation_list(f: &mut Frame, app: &App, area: Rect) {
    if app.conversations.is_empty() {
        let block = Block::default()
            .title(Span::styled(" Conversations ", Theme::title_focused()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border_focused());
        let empty = Paragraph::new(Line::from(vec![
            Span::styled("  Lancez ", Style::default().fg(Theme::subtext0())),
            Span::styled("verchat import --auto", Style::default().fg(Theme::blue()).add_modifier(Modifier::BOLD)),
        ])).block(block);
        f.render_widget(empty, area);
        return;
    }

    let max_width = area.width as usize;
    let mut items: Vec<ListItem> = Vec::new();
    let mut current_date_label = String::new();

    for (i, conv) in app.conversations.iter().enumerate() {
        let date_label = format_date_label(&conv.updated_at);
        if date_label != current_date_label {
            current_date_label = date_label.clone();
            if !items.is_empty() {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("  {date_label}"),
                    Style::default().fg(Theme::subtext0()).add_modifier(Modifier::BOLD),
                ),
            ])));
            items.push(ListItem::new(Line::from("")));
        }

        let source_color = Theme::source_color(&conv.source);
        let is_selected = i == app.selected_index;
        let fav = if conv.favorite { "★ " } else { "  " };

        let meta_len = 20; // source + count
        let title_max = max_width.saturating_sub(meta_len + 6);
        let title = truncate(&clean_title(&conv.title), title_max);

        let source_short = match conv.source {
            Source::ClaudeCode => "CC",
            Source::LmStudio => "LM",
            Source::ContinueDev => "CD",
            Source::Aider => "Ad",
            Source::GeminiCli => "Gm",
            Source::OpenCode => "OC",
            Source::Cursor => "Cu",
            Source::Windsurf => "WS",
        };

        let line = Line::from(vec![
            Span::styled(fav, if conv.favorite { Theme::favorite() } else { Style::default().fg(Theme::surface2()) }),
            Span::styled("● ", Style::default().fg(source_color)),
            Span::styled(
                title,
                if is_selected {
                    Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::subtext1())
                },
            ),
            Span::styled(
                format!("  {source_short:>2}"),
                Style::default().fg(source_color),
            ),
            Span::styled(
                format!(" {:>3}m ", conv.message_count),
                Style::default().fg(Theme::overlay0()),
            ),
        ]);

        let item = if is_selected {
            ListItem::new(line).style(Style::default().bg(Theme::surface0()))
        } else {
            ListItem::new(line)
        };

        items.push(item);
    }

    let total = app.conversations.len();
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" Conversations ", Theme::title_focused()),
            Span::styled(format!("({total}) "), Style::default().fg(Theme::overlay0())),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focused());

    let visual_idx = find_visual_index(app);
    let mut state = ListState::default().with_selected(Some(visual_idx));
    let list = List::new(items).block(block).highlight_symbol("");

    f.render_stateful_widget(list, area, &mut state);

    if total > area.height as usize - 2 {
        let mut scrollbar_state = ScrollbarState::new(total)
            .position(app.selected_index)
            .viewport_content_length(area.height as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(Theme::surface2())),
            area.inner(ratatui::layout::Margin { horizontal: 0, vertical: 1 }),
            &mut scrollbar_state,
        );
    }
}

fn render_status_bar(f: &mut Frame, _app: &App, area: Rect) {
    let bar = Line::from(vec![
        Span::styled(" / ", Theme::status_key()),
        Span::styled("search ", Theme::status_label()),
        Span::styled(" ⏎ ", Theme::status_key()),
        Span::styled("open ", Theme::status_label()),
        Span::styled(" a ", Theme::status_key()),
        Span::styled("add ", Theme::status_label()),
        Span::styled(" s ", Theme::status_key()),
        Span::styled("stats ", Theme::status_label()),
        Span::styled(" q ", Theme::status_key()),
        Span::styled("quit ", Theme::status_label()),
    ]);

    f.render_widget(Paragraph::new(bar).style(Theme::status_bar()), area);
}

// ── Helpers ─────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let end = max.saturating_sub(3);
        format!("{}...", chars[..end].iter().collect::<String>())
    }
}

fn clean_title(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(s)
        .trim()
        .to_string()
}

fn format_date_label(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let today = chrono::Utc::now().date_naive();
    let date = dt.date_naive();
    let diff = today - date;

    if diff.num_days() == 0 {
        "Aujourd'hui".to_string()
    } else if diff.num_days() == 1 {
        "Hier".to_string()
    } else if diff.num_days() < 7 {
        format!("Il y a {} jours", diff.num_days())
    } else {
        dt.format("%d/%m/%Y").to_string()
    }
}

fn find_visual_index(app: &App) -> usize {
    let mut visual = 0;
    let mut current_date = String::new();

    for (i, conv) in app.conversations.iter().enumerate() {
        let date_label = format_date_label(&conv.updated_at);
        if date_label != current_date {
            current_date = date_label;
            if visual > 0 {
                visual += 1;
            }
            visual += 1;
            visual += 1;
        }
        if i == app.selected_index {
            return visual;
        }
        visual += 1;
    }
    visual
}
