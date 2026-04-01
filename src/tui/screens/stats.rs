use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, BorderType, Borders, Padding, Paragraph};

use crate::model::Source;
use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(7),  // Overview stats
        Constraint::Min(1),    // Bar chart
        Constraint::Length(1), // Status bar
    ])
    .split(area);

    render_overview(f, app, layout[0]);
    render_chart(f, app, layout[1]);
    render_status_bar(f, layout[2]);
}

fn render_overview(f: &mut Frame, app: &App, area: Rect) {
    let total = app.conversations.len();
    let total_messages: usize = app.conversations.iter().map(|c| c.message_count).sum();
    let total_input: u64 = app.conversations.iter().map(|c| c.tokens_input).sum();
    let total_cache_write: u64 = app.conversations.iter().map(|c| c.tokens_cache_write).sum();
    let total_cache_read: u64 = app.conversations.iter().map(|c| c.tokens_cache_read).sum();
    let total_output: u64 = app.conversations.iter().map(|c| c.tokens_output).sum();
    let total_tokens = total_input + total_cache_write + total_cache_read + total_output;

    let by_source = count_by_source(&app.conversations);

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("    Conversations:  ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format!("{total}"),
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("    Messages:  ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format!("{total_messages}"),
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("    Input: ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format_tokens(total_input),
                Style::default().fg(Theme::subtext1()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Cache W: ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format_tokens(total_cache_write),
                Style::default().fg(Theme::peach()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Cache R: ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format_tokens(total_cache_read),
                Style::default().fg(Theme::teal()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Output: ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format_tokens(total_output),
                Style::default().fg(Theme::mauve()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Total: ", Style::default().fg(Theme::subtext0())),
            Span::styled(
                format_tokens(total_tokens),
                Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    // Détail par source sur une ligne
    let source_parts: Vec<Span> = by_source
        .iter()
        .map(|(source, count)| {
            let color = Theme::source_color(source);
            vec![
                Span::styled("  ● ", Style::default().fg(color)),
                Span::styled(
                    format!("{}: ", source.display_name()),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{count}"),
                    Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD),
                ),
            ]
        })
        .flatten()
        .collect();

    lines.push(Line::from(
        [vec![Span::styled("   ", Style::default())], source_parts].concat(),
    ));

    let block = Block::default()
        .title(Span::styled(" Statistiques ", Theme::title_focused()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border_focused())
        .padding(Padding::horizontal(1));

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_chart(f: &mut Frame, app: &App, area: Rect) {
    let by_source = count_by_source(&app.conversations);

    if by_source.is_empty() {
        return;
    }

    let bars: Vec<Bar> = by_source
        .iter()
        .map(|(source, count)| {
            let color = Theme::source_color(source);
            Bar::default()
                .value(*count as u64)
                .label(Line::from(source.display_name().to_string()))
                .style(Style::default().fg(color))
                .value_style(Style::default().fg(Theme::text()).add_modifier(Modifier::BOLD))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .title(Span::styled(" Conversations par source ", Theme::title_focused()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Theme::border_focused())
                .padding(Padding::new(2, 2, 1, 1)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(14)
        .bar_gap(3)
        .direction(ratatui::layout::Direction::Vertical);

    f.render_widget(chart, area);
}

fn render_status_bar(f: &mut Frame, area: Rect) {
    let bar = Line::from(vec![
        Span::styled(" esc ", Theme::status_key()),
        Span::styled("retour ", Theme::status_label()),
    ]);

    f.render_widget(
        Paragraph::new(bar).style(Theme::status_bar()),
        area,
    );
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

fn count_by_source(
    conversations: &[crate::store::sqlite::ConversationSummary],
) -> Vec<(Source, usize)> {
    let sources = [
        Source::ClaudeCode,
        Source::LmStudio,
        Source::ContinueDev,
        Source::Aider,
    ];

    sources
        .iter()
        .filter_map(|source| {
            let count = conversations.iter().filter(|c| c.source == *source).count();
            if count > 0 {
                Some((*source, count))
            } else {
                None
            }
        })
        .collect()
}
