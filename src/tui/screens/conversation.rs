use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

use crate::model::Role;
use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::horizontal([
        Constraint::Length(20), // Sidebar info
        Constraint::Min(1),    // Messages
    ])
    .split(area);

    render_sidebar(f, app, layout[0]);
    render_main(f, app, layout[1]);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let conv = match &app.current_conversation {
        Some(c) => c,
        None => return,
    };

    let source_color = Theme::source_color(&conv.source);
    let source_name = conv.source.to_string();
    let model = conv.model.as_deref().unwrap_or("unknown");
    let date = conv.created_at.format("%d/%m/%Y").to_string();
    let time = conv.created_at.format("%H:%M").to_string();
    let msg_count = conv.messages.len();
    let user_count = conv.messages.iter().filter(|m| m.role == Role::User).count();
    let assistant_count = conv.messages.iter().filter(|m| m.role == Role::Assistant).count();

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" ● ", Style::default().fg(source_color)),
            Span::styled(&source_name, Style::default().fg(source_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Model", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {}", truncate(model, 16)), Style::default().fg(Theme::subtext1())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Date", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {date}"), Style::default().fg(Theme::subtext1())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {time}"), Style::default().fg(Theme::subtext1())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Messages", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {msg_count} total"), Style::default().fg(Theme::subtext1())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {user_count}"), Style::default().fg(Theme::green())),
            Span::styled(" user", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {assistant_count}"), Style::default().fg(Theme::mauve())),
            Span::styled(" assistant", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Tokens", Style::default().fg(Theme::overlay0())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} input", format_tokens(conv.tokens_input)), Style::default().fg(Theme::subtext1())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} cache w", format_tokens(conv.tokens_cache_write)), Style::default().fg(Theme::peach())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} cache r", format_tokens(conv.tokens_cache_read)), Style::default().fg(Theme::teal())),
        ]),
        Line::from(vec![
            Span::styled(format!(" {} output", format_tokens(conv.tokens_output)), Style::default().fg(Theme::mauve())),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(" ──────────", Style::default().fg(Theme::surface2()))]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" c ", Theme::status_key()),
            Span::styled(" copier", Style::default().fg(Theme::subtext0())),
        ]),
        Line::from(vec![
            Span::styled(" l ", Theme::status_key()),
            Span::styled(" lancer", Style::default().fg(Theme::subtext0())),
        ]),
    ];

    let block = Block::default()
        .title(Span::styled(" Info ", Theme::title_focused()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_sidebar());

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_main(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Min(1),   // Messages
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    render_messages(f, app, layout[0]);
    render_status_bar(f, layout[1]);
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let conv = match &app.current_conversation {
        Some(c) => c,
        None => return,
    };

    let content_width = area.width.saturating_sub(6) as usize;
    let mut lines: Vec<Line> = Vec::new();

    for (idx, msg) in conv.messages.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}", "─".repeat(content_width.min(60))),
                    Style::default().fg(Theme::surface1()),
                ),
            ]));
            lines.push(Line::from(""));
        }

        let (role_style, icon) = match msg.role {
            Role::User => (Theme::role_user(), "▸"),
            Role::Assistant => (Theme::role_assistant(), "◂"),
            Role::System => (Theme::role_system(), "⚙"),
        };

        let ts = msg.timestamp
            .map(|t| format!("  {}", t.format("%H:%M")))
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::styled(format!("  {icon} "), role_style),
            Span::styled(msg.role.to_string(), role_style),
            Span::styled(ts, Theme::timestamp()),
        ]));
        lines.push(Line::from(""));

        let content_style = match msg.role {
            Role::User => Style::default().fg(Theme::text()),
            Role::Assistant => Style::default().fg(Theme::subtext1()),
            Role::System => Style::default().fg(Theme::overlay0()),
        };

        let gutter_style = match msg.role {
            Role::User => Style::default().fg(Theme::green()),
            Role::Assistant => Style::default().fg(Theme::mauve()),
            Role::System => Style::default().fg(Theme::surface2()),
        };

        let max_lines = 80;
        let content_lines: Vec<&str> = msg.content.lines().collect();
        let show = content_lines.len().min(max_lines);

        for content_line in &content_lines[..show] {
            lines.push(Line::from(vec![
                Span::styled("  ┃ ", gutter_style),
                Span::styled(*content_line, content_style),
            ]));
        }

        if content_lines.len() > max_lines {
            lines.push(Line::from(vec![
                Span::styled("  ┃ ", gutter_style),
                Span::styled(
                    format!("  ... {} lignes supplémentaires", content_lines.len() - max_lines),
                    Style::default().fg(Theme::overlay0()),
                ),
            ]));
        }
    }

    let total_lines = lines.len() as u16;
    let visible = area.height.saturating_sub(2);

    let title_text = conv.title.chars().take(area.width as usize - 4).collect::<String>();
    let title_text = clean_title(&title_text);

    let block = Block::default()
        .title(Span::styled(format!(" {title_text} "), Theme::title_focused()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focused())
        .padding(Padding::new(1, 1, 0, 0));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);

    if total_lines > visible {
        let mut scrollbar_state = ScrollbarState::new(total_lines as usize)
            .position(app.scroll_offset as usize)
            .viewport_content_length(visible as usize);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(Theme::surface2())),
            area.inner(Margin { horizontal: 0, vertical: 1 }),
            &mut scrollbar_state,
        );
    }
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let bar = Line::from(vec![
        Span::styled(" esc ", Theme::status_key()),
        Span::styled("retour ", Theme::status_label()),
        Span::styled(" ↑↓ ", Theme::status_key()),
        Span::styled("scroll ", Theme::status_label()),
        Span::styled(" c ", Theme::status_key()),
        Span::styled("copier ", Theme::status_label()),
        Span::styled(" l ", Theme::status_key()),
        Span::styled("lancer ", Theme::status_label()),
    ]);

    f.render_widget(Paragraph::new(bar).style(Theme::status_bar()), area);
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

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}
