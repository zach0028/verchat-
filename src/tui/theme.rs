use ratatui::style::{Color, Modifier, Style};

use crate::model::Source;

/// Palette custom VER.CHAT — bleu profond, cyan, violet.
/// Inspirée de GitHub Dark avec une identité propre.
pub struct Theme;

impl Theme {
    // ── Surfaces ──────────────────────────────────────────

    /// Fond principal.
    pub fn base() -> Color {
        Color::Rgb(13, 17, 23) // #0d1117
    }

    /// Surface (panneaux, cartes).
    pub fn surface0() -> Color {
        Color::Rgb(22, 27, 34) // #161b22
    }

    /// Surface hover / sélection.
    pub fn surface1() -> Color {
        Color::Rgb(33, 38, 45) // #21262d
    }

    /// Surface active.
    pub fn surface2() -> Color {
        Color::Rgb(48, 54, 61) // #30363d
    }

    /// Fond sidebar / barre de statut.
    pub fn mantle() -> Color {
        Color::Rgb(1, 4, 9) // #010409
    }

    // ── Texte ─────────────────────────────────────────────

    /// Texte principal.
    pub fn text() -> Color {
        Color::Rgb(230, 237, 243) // #e6edf3
    }

    /// Texte secondaire.
    pub fn subtext1() -> Color {
        Color::Rgb(201, 209, 217) // #c9d1d9
    }

    /// Texte tertiaire.
    pub fn subtext0() -> Color {
        Color::Rgb(139, 148, 158) // #8b949e
    }

    /// Texte muted / placeholder.
    pub fn overlay0() -> Color {
        Color::Rgb(110, 118, 129) // #6e7681
    }

    /// Bordures subtiles.
    pub fn overlay1() -> Color {
        Color::Rgb(139, 148, 158) // #8b949e
    }

    // ── Accents ───────────────────────────────────────────

    /// Accent primaire — Cyan. Focus, liens, actions.
    pub fn blue() -> Color {
        Color::Rgb(88, 166, 255) // #58a6ff
    }

    /// Accent secondaire — Violet. IA, assistant.
    pub fn mauve() -> Color {
        Color::Rgb(188, 140, 255) // #bc8cff
    }

    /// Vert — succès, user, validation.
    pub fn green() -> Color {
        Color::Rgb(63, 185, 80) // #3fb950
    }

    /// Orange — warnings, accents chauds.
    pub fn peach() -> Color {
        Color::Rgb(210, 153, 34) // #d29922
    }

    /// Rouge — erreurs, destructif.
    pub fn red() -> Color {
        Color::Rgb(248, 81, 73) // #f85149
    }

    /// Jaune — favoris, attention.
    pub fn yellow() -> Color {
        Color::Rgb(210, 153, 34) // #d29922
    }

    /// Teal — accent frais.
    pub fn teal() -> Color {
        Color::Rgb(63, 185, 164) // #3fb9a4
    }

    /// Rose — accent vif.
    pub fn pink() -> Color {
        Color::Rgb(219, 97, 162) // #db61a2
    }

    // ── Couleurs des sources ──────────────────────────────

    pub fn source_color(source: &Source) -> Color {
        match source {
            Source::ClaudeCode => Self::mauve(),  // Violet — Claude
            Source::LmStudio => Self::blue(),     // Cyan — LM Studio
            Source::ContinueDev => Self::green(), // Vert — Continue.dev
            Source::Aider => Self::peach(),       // Orange — Aider
            Source::GeminiCli => Self::teal(),    // Teal — Gemini
            Source::OpenCode => Self::yellow(),   // Jaune — OpenCode
            Source::Cursor => Self::pink(),       // Rose — Cursor
            Source::Windsurf => Self::blue(),     // Cyan (alt) — Windsurf
        }
    }

    // ── Styles composés ───────────────────────────────────

    pub fn title_focused() -> Style {
        Style::default().fg(Self::blue()).add_modifier(Modifier::BOLD)
    }

    pub fn title_unfocused() -> Style {
        Style::default().fg(Self::overlay0())
    }

    pub fn border_focused() -> Style {
        Style::default().fg(Self::surface2())
    }

    pub fn border_unfocused() -> Style {
        Style::default().fg(Self::surface2())
    }

    /// Bordure de la sidebar.
    pub fn border_sidebar() -> Style {
        Style::default().fg(Self::surface2())
    }

    pub fn list_selected() -> Style {
        Style::default()
            .bg(Self::surface1())
            .fg(Self::text())
            .add_modifier(Modifier::BOLD)
    }

    pub fn list_normal() -> Style {
        Style::default().fg(Self::subtext1())
    }

    pub fn placeholder() -> Style {
        Style::default().fg(Self::overlay0())
    }

    pub fn favorite() -> Style {
        Style::default().fg(Self::yellow()).add_modifier(Modifier::BOLD)
    }

    pub fn status_bar() -> Style {
        Style::default().bg(Self::mantle()).fg(Self::subtext0())
    }

    pub fn status_key() -> Style {
        Style::default()
            .bg(Self::surface2())
            .fg(Self::text())
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_label() -> Style {
        Style::default().bg(Self::mantle()).fg(Self::subtext0())
    }

    pub fn role_user() -> Style {
        Style::default().fg(Self::green()).add_modifier(Modifier::BOLD)
    }

    pub fn role_assistant() -> Style {
        Style::default().fg(Self::mauve()).add_modifier(Modifier::BOLD)
    }

    pub fn role_system() -> Style {
        Style::default().fg(Self::overlay0()).add_modifier(Modifier::ITALIC)
    }

    pub fn timestamp() -> Style {
        Style::default().fg(Self::overlay0())
    }

    pub fn search_highlight() -> Style {
        Style::default()
            .fg(Self::base())
            .bg(Self::yellow())
            .add_modifier(Modifier::BOLD)
    }

    /// Style pour un item de sidebar sélectionné.
    pub fn sidebar_selected() -> Style {
        Style::default()
            .fg(Self::text())
            .bg(Self::surface1())
            .add_modifier(Modifier::BOLD)
    }

    /// Style pour un item de sidebar normal.
    pub fn sidebar_normal() -> Style {
        Style::default().fg(Self::subtext0())
    }

    /// Style pour le compteur dans la sidebar.
    pub fn sidebar_count() -> Style {
        Style::default().fg(Self::overlay0())
    }
}
