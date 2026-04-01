use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(3), // Search input
        Constraint::Min(1),   // Results
        Constraint::Length(1), // Status bar (slim)
    ])
    .split(area);

    render_search_input(f, app, layout[0]);
    render_results(f, app, layout[1]);
    render_status_bar(f, app, layout[2]);
}

fn render_search_input(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.search_focused;
    let border_style = if is_focused { Theme::border_focused() } else { Theme::border_unfocused() };
    let title_style = if is_focused { Theme::title_focused() } else { Theme::title_unfocused() };

    let block = Block::default()
        .title(Span::styled(" Recherche ", title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .padding(Padding::horizontal(1));

    let cursor = if is_focused { "│" } else { "" };
    let content = if app.search_query.is_empty() {
        if is_focused {
            Line::from(vec![
                Span::styled("  /  ", Style::default().fg(Theme::blue())),
                Span::styled(cursor, Style::default().fg(Theme::blue())),
            ])
        } else {
            Line::from(Span::styled(
                "  Tapez votre recherche...",
                Theme::placeholder(),
            ))
        }
    } else {
        Line::from(vec![
            Span::styled("  /  ", Style::default().fg(Theme::blue())),
            Span::styled(&app.search_query, Style::default().fg(Theme::text())),
            Span::styled(cursor, Style::default().fg(Theme::blue())),
        ])
    };

    f.render_widget(Paragraph::new(content).block(block), area);
}

fn render_results(f: &mut Frame, app: &App, area: Rect) {
    if app.search_query.is_empty() {
        let block = Block::default()
            .title(Span::styled(" Résultats ", Theme::title_unfocused()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border_unfocused());

        let hint = Paragraph::new(Line::from(vec![
            Span::styled(
                "  Commencez à taper pour rechercher",
                Style::default().fg(Theme::overlay0()),
            ),
        ]))
        .block(block);
        f.render_widget(hint, area);
        return;
    }

    if app.search_results.is_empty() {
        let block = Block::default()
            .title(Span::styled(" 0 résultat ", Theme::title_unfocused()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Theme::border_unfocused());

        let msg = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  Aucun résultat pour \"{}\"", app.search_query),
                Style::default().fg(Theme::subtext0()),
            ),
        ]))
        .block(block);
        f.render_widget(msg, area);
        return;
    }

    let max_width = area.width as usize;
    let mut items: Vec<ListItem> = Vec::new();

    for (i, result) in app.search_results.iter().enumerate() {
        let c = &result.conversation;
        let source_color = Theme::source_color(&c.source);
        let is_selected = i == app.search_selected;

        let source_name = c.source.display_name();
        let title = truncate(&clean_title(&c.title), max_width.saturating_sub(30));

        // Ligne 1 : titre
        let title_line = Line::from(vec![
            Span::styled("  ● ", Style::default().fg(source_color)),
            Span::styled(
                title.clone(),
                if is_selected {
                    Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::subtext1())
                },
            ),
            Span::styled(format!("  {source_name}"), Style::default().fg(source_color)),
            Span::styled(format!("  {}m", c.message_count), Style::default().fg(Theme::overlay0())),
        ]);

        // Ligne 2 : snippet nettoyé
        let snippet = clean_snippet(&result.snippet, max_width.saturating_sub(8));
        let snippet_line = Line::from(vec![
            Span::styled(format!("    {snippet}"), Style::default().fg(Theme::overlay1())),
        ]);

        let item = if is_selected {
            ListItem::new(vec![title_line, snippet_line, Line::from("")])
                .style(Style::default().bg(Theme::surface0()))
        } else {
            ListItem::new(vec![title_line, snippet_line, Line::from("")])
        };

        items.push(item);
    }

    let count = app.search_results.len();
    let block = Block::default()
        .title(Span::styled(
            format!(" {} résultat{} ", count, if count > 1 { "s" } else { "" }),
            Theme::title_focused(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focused());

    f.render_widget(List::new(items).block(block), area);
}

fn render_status_bar(f: &mut Frame, _app: &App, area: Rect) {
    let bar = Line::from(vec![
        Span::styled(" esc ", Theme::status_key()),
        Span::styled("retour ", Theme::status_label()),
        Span::styled(" ⏎ ", Theme::status_key()),
        Span::styled("ouvrir ", Theme::status_label()),
        Span::styled(" ↑↓ ", Theme::status_key()),
        Span::styled("naviguer ", Theme::status_label()),
    ]);

    f.render_widget(
        Paragraph::new(bar).style(Theme::status_bar()),
        area,
    );
}

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

fn clean_snippet(s: &str, max: usize) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c == '\n' || c.is_control() { ' ' } else { c })
        .collect();
    truncate(cleaned.trim(), max)
}
