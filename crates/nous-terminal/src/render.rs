//! Platform-agnostic rendering types.
//!
//! These types represent the terminal's visual state in a way that any
//! renderer (ratatui, browser canvas, Tauri webview, WASM) can consume.
//! The Infinite Minimalism palette is the default.

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

/// Terminal color theme following Infinite Minimalism.
///
/// Deep blacks, near-white text, warm gold accent.
/// European luxury: restrained, confident, quiet.
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
        Self {
            // Infinite Minimalism: deep black canvas
            background: Color::Rgb(0, 0, 0),
            foreground: Color::Rgb(224, 224, 224),
            cursor: Color::Rgb(212, 175, 55),  // warm gold accent
            selection: Color::Rgb(40, 40, 40), // subtle highlight

            // Muted, desaturated ANSI — not garish
            black: Color::Rgb(0, 0, 0),
            red: Color::Rgb(190, 80, 70),
            green: Color::Rgb(100, 180, 100),
            yellow: Color::Rgb(212, 175, 55), // gold
            blue: Color::Rgb(90, 140, 200),
            magenta: Color::Rgb(160, 110, 180),
            cyan: Color::Rgb(80, 180, 180),
            white: Color::Rgb(200, 200, 200),
            bright_black: Color::Rgb(80, 80, 80),
            bright_red: Color::Rgb(220, 110, 100),
            bright_green: Color::Rgb(130, 210, 130),
            bright_yellow: Color::Rgb(240, 210, 90),
            bright_blue: Color::Rgb(120, 170, 230),
            bright_magenta: Color::Rgb(190, 140, 210),
            bright_cyan: Color::Rgb(110, 210, 210),
            bright_white: Color::Rgb(240, 240, 240),
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
        assert_eq!(theme.black, Color::Rgb(0, 0, 0));
        assert_eq!(theme.yellow, Color::Rgb(212, 175, 55)); // gold accent
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
