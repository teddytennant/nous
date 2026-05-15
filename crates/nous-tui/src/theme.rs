//! TUI theme — a thin facade over [`nous_design::palette`].
//!
//! Editorial: ink and ivory, a single oxblood accent used like a stamp,
//! sage/clay for state. Hairlines, not borders. Hierarchy comes from weight,
//! tracking (in spirit), and `meta` casing — never from boxes or chips.
//!
//! Public method names are stable so call-sites in `widgets`/`views` keep
//! working: `base`, `dim`, `accent`, `bold`, `title`, `selected`, `border`,
//! `error`, `success`. Two new helpers — `meta` and `rule` — expose the
//! editorial-specific tokens (stone-foreground metadata, hairline rule).
use nous_design::palette::tui as p;
use ratatui::style::{Color, Modifier, Style};

/// Editorial TUI theme.
pub struct Theme;

impl Theme {
    // ── Re-exposed canonical tokens ────────────────────────────────────────
    /// Primary surface — `INK`.
    pub const BG: Color = p::INK;
    /// Primary text — `IVORY`.
    pub const FG: Color = p::IVORY;
    /// Secondary text — `IVORY_DIM`.
    pub const FG_DIM: Color = p::IVORY_DIM;
    /// Tertiary / metadata — `STONE`.
    pub const META: Color = p::STONE;
    /// 1px hairline — `RULE`.
    pub const RULE: Color = p::RULE;
    /// Single accent — `OXBLOOD`.
    pub const ACCENT: Color = p::OXBLOOD;
    /// Pressed / visited oxblood.
    pub const ACCENT_DIM: Color = p::OXBLOOD_DIM;
    /// Positive state — `SAGE`.
    pub const POS: Color = p::SAGE;
    /// Warning state — `CLAY`.
    pub const WARN: Color = p::CLAY;
    /// Raised surface (modal backgrounds, focused row marker).
    pub const BG_RAISED: Color = p::INK_2;

    // ── Backwards-compatible aliases (kept so older call-sites compile) ────
    /// Alias for [`META`]. Deprecated — call sites should prefer `meta()`.
    pub const DIM: Color = p::STONE;
    /// Hairline color, surfaced as the legacy `BORDER` constant.
    pub const BORDER: Color = p::RULE;
    /// Error / clay foreground.
    pub const ERROR: Color = p::CLAY;
    /// Positive / sage foreground.
    pub const SUCCESS: Color = p::SAGE;
    /// Subtle row highlight — raised surface.
    pub const SELECTION: Color = p::INK_2;

    // ── Style helpers ──────────────────────────────────────────────────────

    /// Body text on the page surface.
    pub fn base() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG)
    }

    /// Dimmed body — for de-emphasized inline content. Kept for back-compat;
    /// new code should prefer `meta()` for stone metadata.
    pub fn dim() -> Style {
        Style::default().fg(Self::FG_DIM).bg(Self::BG)
    }

    /// Tertiary metadata — stone, no decoration. DIDs, captions, peer counts,
    /// uppercase meta-caps labels.
    pub fn meta() -> Style {
        Style::default().fg(Self::META).bg(Self::BG)
    }

    /// 1px hairline rule color (rule on ink). Used for `Borders::TOP` /
    /// `Borders::BOTTOM` strokes.
    pub fn rule() -> Style {
        Style::default().fg(Self::RULE).bg(Self::BG)
    }

    /// Single accent — oxblood, no fill. Use sparingly: actions, the active
    /// nav mark, focus rings, the dot beside a live count.
    pub fn accent() -> Style {
        Style::default().fg(Self::ACCENT).bg(Self::BG)
    }

    /// Body weight — bold ivory. Restraint: most "hierarchy" lives in caps
    /// and tracking, not bolding.
    pub fn bold() -> Style {
        Style::default()
            .fg(Self::FG)
            .bg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Title / wordmark — bold ivory (the wordmark is *type*, not the
    /// accent). The accent is reserved for marks, not headings.
    pub fn title() -> Style {
        Style::default()
            .fg(Self::FG)
            .bg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Selected row indicator — ivory body kept on the page surface (the
    /// oxblood `▌` margin bar lives in widgets/views; the row itself is
    /// never washed with a fill).
    pub fn selected() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG)
    }

    /// Hairline border foreground (legacy alias of [`rule()`]).
    pub fn border() -> Style {
        Self::rule()
    }

    /// Error tone — clay, bold. Warm. Never an alarm-red.
    pub fn error() -> Style {
        Style::default()
            .fg(Self::ERROR)
            .bg(Self::BG)
            .add_modifier(Modifier::BOLD)
    }

    /// Success / live tone — sage, no decoration.
    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS).bg(Self::BG)
    }

    /// Status bar — stone foreground on the page surface (no chrome bar fill).
    pub fn status_bar() -> Style {
        Style::default().fg(Self::META).bg(Self::BG)
    }

    /// Input row — ivory text on raised surface (subtle separation by tone,
    /// not by border).
    pub fn input() -> Style {
        Style::default().fg(Self::FG).bg(Self::BG_RAISED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_uses_design_tokens() {
        // ink #0E0E0C
        assert_eq!(Theme::BG, Color::Rgb(0x0E, 0x0E, 0x0C));
        // ivory #EFEAE0
        assert_eq!(Theme::FG, Color::Rgb(0xEF, 0xEA, 0xE0));
        // oxblood #B23A3A
        assert_eq!(Theme::ACCENT, Color::Rgb(0xB2, 0x3A, 0x3A));
        // stone #6F6A60
        assert_eq!(Theme::META, Color::Rgb(0x6F, 0x6A, 0x60));
        // rule #1F1D1A
        assert_eq!(Theme::RULE, Color::Rgb(0x1F, 0x1D, 0x1A));
    }

    #[test]
    fn styles_have_correct_fg() {
        assert_eq!(Theme::base().fg, Some(Theme::FG));
        assert_eq!(Theme::meta().fg, Some(Theme::META));
        assert_eq!(Theme::accent().fg, Some(Theme::ACCENT));
        assert_eq!(Theme::rule().fg, Some(Theme::RULE));
    }

    #[test]
    fn title_is_bold() {
        assert!(Theme::title().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn no_warm_gold_anywhere() {
        // the legacy gold hex must be gone
        let gold = Color::Rgb(212, 175, 55);
        assert_ne!(Theme::ACCENT, gold);
        assert_ne!(Theme::FG, gold);
        assert_ne!(Theme::META, gold);
    }
}
