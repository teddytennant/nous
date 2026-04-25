//! Color tokens.
//!
//! Each constant is an `(r, g, b)` triple in 8-bit sRGB. Hex literals are
//! preserved in the doc comment for easy cross-referencing with
//! `docs/design/tokens.json` and the web CSS variables.
//!
//! With the `ratatui` feature, the [`tui`] submodule re-exposes every token as
//! a [`ratatui::style::Color`] plus a small set of opinionated [`ratatui::style::Style`]
//! helpers (`accent_style`, `meta_style`, `error_style`, `success_style`).

// ── Dark mode (default surface) ─────────────────────────────────────────────

/// `#0E0E0C` — primary surface (the page).
pub const INK: (u8, u8, u8) = (0x0E, 0x0E, 0x0C);

/// `#161613` — raised surface (cards, panels).
pub const INK_2: (u8, u8, u8) = (0x16, 0x16, 0x13);

/// `#EFEAE0` — primary text on dark surface.
pub const IVORY: (u8, u8, u8) = (0xEF, 0xEA, 0xE0);

/// `#C9C3B6` — secondary text.
pub const IVORY_DIM: (u8, u8, u8) = (0xC9, 0xC3, 0xB6);

/// `#6F6A60` — tertiary text, metadata, captions.
pub const STONE: (u8, u8, u8) = (0x6F, 0x6A, 0x60);

/// `#1F1D1A` — 1px hairline rules.
pub const RULE: (u8, u8, u8) = (0x1F, 0x1D, 0x1A);

/// `#B23A3A` — single accent: actions, marks, focus rings.
pub const OXBLOOD: (u8, u8, u8) = (0xB2, 0x3A, 0x3A);

/// `#7A2A2A` — pressed / visited variant of [`OXBLOOD`].
pub const OXBLOOD_DIM: (u8, u8, u8) = (0x7A, 0x2A, 0x2A);

/// `#8FA48A` — positive state (success, on-line, confirmed).
pub const SAGE: (u8, u8, u8) = (0x8F, 0xA4, 0x8A);

/// `#C2785A` — warning state (degraded, attention).
pub const CLAY: (u8, u8, u8) = (0xC2, 0x78, 0x5A);

// ── Light mode (variant) ────────────────────────────────────────────────────

/// `#F4F1EA` — light primary surface.
pub const PAPER: (u8, u8, u8) = (0xF4, 0xF1, 0xEA);

/// `#14130F` — primary text on light surface.
pub const INK_TEXT: (u8, u8, u8) = (0x14, 0x13, 0x0F);

/// `#5A564E` — light-mode secondary text.
pub const IVORY_DIM_LIGHT: (u8, u8, u8) = (0x5A, 0x56, 0x4E);

/// `#8A8478` — light-mode tertiary text.
pub const STONE_LIGHT: (u8, u8, u8) = (0x8A, 0x84, 0x78);

/// `#DCD7CC` — light-mode hairline rules.
pub const HAIRLINE: (u8, u8, u8) = (0xDC, 0xD7, 0xCC);

/// `#6E8468` — light-mode positive state.
pub const SAGE_LIGHT: (u8, u8, u8) = (0x6E, 0x84, 0x68);

/// `#A4604A` — light-mode warning state.
pub const CLAY_LIGHT: (u8, u8, u8) = (0xA4, 0x60, 0x4A);

// Oxblood is unchanged across modes; reuse [`OXBLOOD`] / [`OXBLOOD_DIM`].

// ── ratatui bindings ────────────────────────────────────────────────────────

/// ratatui-flavored re-export of the palette.
///
/// Each token here is a [`ratatui::style::Color::Rgb`] paired with the same name
/// as the `(u8, u8, u8)` triple in the parent module. A handful of opinionated
/// style helpers cover the common cases (accent, metadata, error, success).
#[cfg(feature = "ratatui")]
pub mod tui {
    use ratatui::style::{Color, Modifier, Style};

    const fn rgb(c: (u8, u8, u8)) -> Color {
        Color::Rgb(c.0, c.1, c.2)
    }

    /// Dark primary surface.
    pub const INK: Color = rgb(super::INK);
    /// Raised dark surface.
    pub const INK_2: Color = rgb(super::INK_2);
    /// Primary text on dark.
    pub const IVORY: Color = rgb(super::IVORY);
    /// Secondary text.
    pub const IVORY_DIM: Color = rgb(super::IVORY_DIM);
    /// Tertiary text / metadata.
    pub const STONE: Color = rgb(super::STONE);
    /// Hairline rules.
    pub const RULE: Color = rgb(super::RULE);
    /// Accent (oxblood).
    pub const OXBLOOD: Color = rgb(super::OXBLOOD);
    /// Pressed / visited oxblood.
    pub const OXBLOOD_DIM: Color = rgb(super::OXBLOOD_DIM);
    /// Positive state.
    pub const SAGE: Color = rgb(super::SAGE);
    /// Warning state.
    pub const CLAY: Color = rgb(super::CLAY);

    /// Light primary surface.
    pub const PAPER: Color = rgb(super::PAPER);
    /// Light-mode primary text.
    pub const INK_TEXT: Color = rgb(super::INK_TEXT);
    /// Light-mode hairline.
    pub const HAIRLINE: Color = rgb(super::HAIRLINE);

    /// Accent style — oxblood foreground, used for actions, marks, focus.
    pub fn accent_style() -> Style {
        Style::default().fg(OXBLOOD)
    }

    /// Metadata style — stone foreground, no decoration. For DIDs, peer
    /// counts, tertiary captions.
    pub fn meta_style() -> Style {
        Style::default().fg(STONE)
    }

    /// Error style — clay foreground (warm, never alarming red).
    pub fn error_style() -> Style {
        Style::default().fg(CLAY).add_modifier(Modifier::BOLD)
    }

    /// Success style — sage foreground.
    pub fn success_style() -> Style {
        Style::default().fg(SAGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_tokens_match_spec() {
        assert_eq!(INK, (0x0E, 0x0E, 0x0C));
        assert_eq!(IVORY, (0xEF, 0xEA, 0xE0));
        assert_eq!(OXBLOOD, (0xB2, 0x3A, 0x3A));
        assert_eq!(OXBLOOD_DIM, (0x7A, 0x2A, 0x2A));
        assert_eq!(SAGE, (0x8F, 0xA4, 0x8A));
        assert_eq!(CLAY, (0xC2, 0x78, 0x5A));
    }

    #[test]
    fn light_tokens_match_spec() {
        assert_eq!(PAPER, (0xF4, 0xF1, 0xEA));
        assert_eq!(INK_TEXT, (0x14, 0x13, 0x0F));
        assert_eq!(HAIRLINE, (0xDC, 0xD7, 0xCC));
    }

    #[cfg(feature = "ratatui")]
    #[test]
    fn ratatui_helpers_use_oxblood() {
        use ratatui::style::Color;
        let style = super::tui::accent_style();
        assert_eq!(style.fg, Some(Color::Rgb(0xB2, 0x3A, 0x3A)));
    }
}
