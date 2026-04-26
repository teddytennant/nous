//! Platform-agnostic rendering types.
//!
//! These types represent the terminal's visual state in a way that any
//! renderer (ratatui, browser canvas, Tauri webview, WASM) can consume.
//! The default theme draws from the editorial palette in
//! [`nous_design::palette`] — ink, ivory, stone, oxblood, sage, clay.

use nous_design::palette;

const fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// A single terminal cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
        }
    }
}

/// Visual style for a terminal cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CellStyle {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            inverse: false,
        }
    }
}

/// Terminal color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

/// A row of cells ready for rendering.
#[derive(Debug, Clone)]
pub struct RenderRow {
    pub cells: Vec<Cell>,
}

/// Terminal color theme following the editorial design language.
///
/// Ink and ivory, a single oxblood accent. The ANSI table is mapped to the
/// canonical palette: oxblood for "red," sage for "green," clay for "yellow,"
/// stone for the muted variants. No teal, no violet, no secondary brand.
#[derive(Debug, Clone)]
pub struct TerminalTheme {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,

    // ANSI 16 colors — muted, sophisticated palette
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        // Editorial: ink/ivory canvas, oxblood mark, sage/clay state.
        Self {
            background: rgb(palette::INK),
            foreground: rgb(palette::IVORY),
            cursor: rgb(palette::OXBLOOD),
            selection: rgb(palette::INK_2),

            // ANSI table mapped onto the eight-token editorial palette.
            // No teal, no violet — magenta/cyan/blue collapse onto stone and
            // ivory_dim so legacy programs still render legibly without
            // introducing colors the design language doesn't allow.
            black: rgb(palette::INK),
            red: rgb(palette::OXBLOOD),
            green: rgb(palette::SAGE),
            yellow: rgb(palette::CLAY),
            blue: rgb(palette::IVORY_DIM),
            magenta: rgb(palette::OXBLOOD_DIM),
            cyan: rgb(palette::STONE),
            white: rgb(palette::IVORY_DIM),
            bright_black: rgb(palette::STONE),
            bright_red: rgb(palette::OXBLOOD),
            bright_green: rgb(palette::SAGE),
            bright_yellow: rgb(palette::CLAY),
            bright_blue: rgb(palette::IVORY),
            bright_magenta: rgb(palette::OXBLOOD),
            bright_cyan: rgb(palette::IVORY_DIM),
            bright_white: rgb(palette::IVORY),
        }
    }
}

impl TerminalTheme {
    /// Resolve an indexed color (0-255) to an RGB color.
    pub fn resolve_color(&self, color: Color) -> Color {
        match color {
            Color::Default => Color::Default,
            Color::Rgb(_, _, _) => color,
            Color::Indexed(idx) => self.indexed_to_rgb(idx),
        }
    }

    fn indexed_to_rgb(&self, idx: u8) -> Color {
        match idx {
            0 => self.black,
            1 => self.red,
            2 => self.green,
            3 => self.yellow,
            4 => self.blue,
            5 => self.magenta,
            6 => self.cyan,
            7 => self.white,
            8 => self.bright_black,
            9 => self.bright_red,
            10 => self.bright_green,
            11 => self.bright_yellow,
            12 => self.bright_blue,
            13 => self.bright_magenta,
            14 => self.bright_cyan,
            15 => self.bright_white,
            // 216 color cube (indices 16..=231)
            16..=231 => {
                let n = idx - 16;
                let b = (n % 6) * 51;
                let g = ((n / 6) % 6) * 51;
                let r = (n / 36) * 51;
                Color::Rgb(r, g, b)
            }
            // 24 grayscale (indices 232..=255)
            _ => {
                let v = 8 + (idx - 232) * 10;
                Color::Rgb(v, v, v)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell() {
        let cell = Cell::default();
        assert_eq!(cell.ch, ' ');
        assert!(!cell.style.bold);
    }

    #[test]
    fn theme_ansi_colors() {
        let theme = TerminalTheme::default();
        // ink (#0E0E0C) is the canvas/black slot
        assert_eq!(theme.black, Color::Rgb(0x0E, 0x0E, 0x0C));
        // oxblood (#B23A3A) is the single accent → "red" slot, also cursor
        assert_eq!(theme.red, Color::Rgb(0xB2, 0x3A, 0x3A));
        assert_eq!(theme.cursor, Color::Rgb(0xB2, 0x3A, 0x3A));
        // sage (#8FA48A) for positive state → "green"
        assert_eq!(theme.green, Color::Rgb(0x8F, 0xA4, 0x8A));
        // clay (#C2785A) for warning → "yellow"
        assert_eq!(theme.yellow, Color::Rgb(0xC2, 0x78, 0x5A));
    }

    #[test]
    fn indexed_color_resolution() {
        let theme = TerminalTheme::default();

        // Standard ANSI
        assert_eq!(theme.resolve_color(Color::Indexed(0)), theme.black);
        assert_eq!(theme.resolve_color(Color::Indexed(1)), theme.red);

        // Color cube: index 16 = (0,0,0)
        assert_eq!(theme.resolve_color(Color::Indexed(16)), Color::Rgb(0, 0, 0));

        // Color cube: index 196 = (5*51, 0, 0) = (255, 0, 0)
        assert_eq!(
            theme.resolve_color(Color::Indexed(196)),
            Color::Rgb(255, 0, 0)
        );

        // Grayscale: index 232 = 8
        assert_eq!(
            theme.resolve_color(Color::Indexed(232)),
            Color::Rgb(8, 8, 8)
        );

        // Grayscale: index 255 = 238
        assert_eq!(
            theme.resolve_color(Color::Indexed(255)),
            Color::Rgb(238, 238, 238)
        );
    }

    #[test]
    fn rgb_passthrough() {
        let theme = TerminalTheme::default();
        let c = Color::Rgb(42, 42, 42);
        assert_eq!(theme.resolve_color(c), c);
    }

    #[test]
    fn default_passthrough() {
        let theme = TerminalTheme::default();
        assert_eq!(theme.resolve_color(Color::Default), Color::Default);
    }
}
