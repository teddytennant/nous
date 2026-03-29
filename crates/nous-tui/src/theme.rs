use ratatui::style::{Color, Modifier, Style};

/// Infinite Minimalism — the TUI design language.
/// Deep blacks, near-white text, one accent color (warm gold).
/// Typography over decoration. Generous whitespace.
pub struct Theme;

impl Theme {
    // Primary palette
    pub const BG: Color = Color::Rgb(8, 8, 8);
    pub const FG: Color = Color::Rgb(230, 230, 225);
    pub const DIM: Color = Color::Rgb(100, 100, 95);
    pub const ACCENT: Color = Color::Rgb(212, 175, 55); // warm gold
    pub const BORDER: Color = Color::Rgb(40, 40, 38);
    pub const ERROR: Color = Color::Rgb(200, 60, 60);
    pub const SUCCESS: Color = Color::Rgb(60, 180, 75);
    pub const SELECTION: Color = Color::Rgb(30, 30, 28);

    pub fn base() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG)
    }

    pub fn dim() -> Style {
        Style::default().fg(Self::DIM).bg(Self::BG)
    }

    pub fn accent() -> Style {
        Style::default().fg(Self::ACCENT).bg(Self::BG)
    }

    pub fn bold() -> Style {
        Style::default()
            .fg(Self::FG)
            .bg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .bg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    pub fn selected() -> Style {
        Style::default().fg(Self::FG).bg(Self::SELECTION)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::BORDER).bg(Self::BG)
    }

    pub fn error() -> Style {
        Style::default().fg(Self::ERROR).bg(Self::BG)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS).bg(Self::BG)
    }

    pub fn status_bar() -> Style {
        Style::default().fg(Self::DIM).bg(Color::Rgb(15, 15, 13))
    }

    pub fn input() -> Style {
        Style::default().fg(Self::FG).bg(Color::Rgb(20, 20, 18))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colors_are_distinct() {
        assert_ne!(format!("{:?}", Theme::BG), format!("{:?}", Theme::FG));
        assert_ne!(format!("{:?}", Theme::ACCENT), format!("{:?}", Theme::DIM));
    }

    #[test]
    fn styles_have_correct_fg() {
        let base = Theme::base();
        assert_eq!(base.fg, Some(Theme::FG));
    }

    #[test]
    fn title_is_bold() {
        let title = Theme::title();
        assert!(title.add_modifier.contains(Modifier::BOLD));
    }
}
