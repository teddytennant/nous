//! VT state machine wrapping the vt100-ctt crate.
//!
//! Converts raw byte streams from a PTY into structured terminal state:
//! screen contents, cursor position, scrollback, and title.
//! Architecture allows swapping vt100-ctt for libghostty-vt in the future.

use crate::render::{Cell, CellStyle, Color, RenderRow, TerminalTheme};

/// Callbacks that capture OSC title changes.
#[derive(Debug, Default)]
struct TitleCapture {
    title: String,
}

impl vt100_ctt::Callbacks for TitleCapture {
    fn set_window_title(&mut self, _screen: &mut vt100_ctt::Screen, title: &[u8]) {
        self.title = String::from_utf8_lossy(title).to_string();
    }
}

/// Terminal state machine.
///
/// Processes VT escape sequences and maintains the full screen buffer,
/// cursor state, scrollback history, and terminal title.
pub struct TerminalState {
    parser: vt100_ctt::Parser<TitleCapture>,
}

impl TerminalState {
    /// Create a new terminal state with the given dimensions.
    pub fn new(rows: u16, cols: u16, scrollback: usize) -> Self {
        Self {
            parser: vt100_ctt::Parser::new_with_callbacks(
                rows,
                cols,
                scrollback,
                TitleCapture::default(),
            ),
        }
    }

    /// Process raw bytes through the VT parser.
    pub fn process(&mut self, data: &[u8]) {
        self.parser.process(data);
    }

    /// Resize the terminal state.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(rows, cols);
    }

    /// Get cursor position as (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Get the terminal title (set via OSC escape sequences).
    pub fn title(&self) -> &str {
        &self.parser.callbacks().title
    }

    /// Whether the cursor should be visible.
    pub fn cursor_visible(&self) -> bool {
        !self.parser.screen().hide_cursor()
    }

    /// Whether the terminal is in alternate screen mode.
    pub fn alternate_screen(&self) -> bool {
        self.parser.screen().alternate_screen()
    }

    /// Get the contents of a specific cell.
    pub fn cell(&self, row: u16, col: u16) -> Cell {
        let screen = self.parser.screen();
        match screen.cell(row, col) {
            Some(c) => {
                let ch = if c.has_contents() {
                    c.contents().chars().next().unwrap_or(' ')
                } else {
                    ' '
                };
                Cell {
                    ch,
                    style: convert_cell_style(c),
                }
            }
            None => Cell::default(),
        }
    }

    /// Render the visible screen as a vector of rows.
    pub fn screen(&self, theme: &TerminalTheme) -> Vec<RenderRow> {
        let screen = self.parser.screen();
        let (rows, cols) = screen.size();
        let mut result = Vec::with_capacity(rows as usize);

        for row in 0..rows {
            let mut cells = Vec::with_capacity(cols as usize);
            for col in 0..cols {
                let cell = match screen.cell(row, col) {
                    Some(c) => {
                        let ch = if c.has_contents() {
                            c.contents().chars().next().unwrap_or(' ')
                        } else {
                            ' '
                        };
                        let mut style = convert_cell_style(c);
                        style.fg = theme.resolve_color(style.fg);
                        style.bg = theme.resolve_color(style.bg);
                        Cell { ch, style }
                    }
                    None => Cell::default(),
                };
                cells.push(cell);
            }
            result.push(RenderRow { cells });
        }

        result
    }

    /// Get the text contents of a row (for testing and accessibility).
    pub fn row_text(&self, row: u16) -> String {
        let screen = self.parser.screen();
        let (_, cols) = screen.size();
        let mut text = String::with_capacity(cols as usize);

        for col in 0..cols {
            if let Some(c) = screen.cell(row, col) {
                if c.has_contents() {
                    text.push_str(c.contents());
                } else {
                    text.push(' ');
                }
            } else {
                text.push(' ');
            }
        }

        text
    }
}

/// Convert a vt100-ctt cell's attributes to our style type.
fn convert_cell_style(cell: &vt100_ctt::Cell) -> CellStyle {
    let fg = convert_color(cell.fgcolor());
    let bg = convert_color(cell.bgcolor());

    CellStyle {
        fg,
        bg,
        bold: cell.bold(),
        italic: cell.italic(),
        underline: cell.underline(),
        strikethrough: false,
        inverse: cell.inverse(),
    }
}

/// Convert a vt100-ctt color to our color type.
fn convert_color(color: vt100_ctt::Color) -> Color {
    match color {
        vt100_ctt::Color::Default => Color::Default,
        vt100_ctt::Color::Idx(idx) => Color::Indexed(idx),
        vt100_ctt::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_state() {
        let state = TerminalState::new(24, 80, 1000);
        let (row, col) = state.cursor_position();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn process_plain_text() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"Hello, Nous!");

        let text = state.row_text(0);
        assert!(text.starts_with("Hello, Nous!"));
    }

    #[test]
    fn process_newline() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"line 1\r\nline 2");

        assert!(state.row_text(0).starts_with("line 1"));
        assert!(state.row_text(1).starts_with("line 2"));
    }

    #[test]
    fn cursor_movement() {
        let mut state = TerminalState::new(24, 80, 1000);
        // ESC[5;10H = move cursor to row 5, col 10 (1-indexed)
        state.process(b"\x1b[5;10H");
        let (row, col) = state.cursor_position();
        assert_eq!(row, 4); // 0-indexed
        assert_eq!(col, 9);
    }

    #[test]
    fn bold_text() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"\x1b[1mBold");

        let cell = state.cell(0, 0);
        assert_eq!(cell.ch, 'B');
        assert!(cell.style.bold);
    }

    #[test]
    fn color_text() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"\x1b[31mRed");

        let cell = state.cell(0, 0);
        assert_eq!(cell.ch, 'R');
        assert_eq!(cell.style.fg, Color::Indexed(1));
    }

    #[test]
    fn rgb_color() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"\x1b[38;2;255;128;0mOrange");

        let cell = state.cell(0, 0);
        assert_eq!(cell.ch, 'O');
        assert_eq!(cell.style.fg, Color::Rgb(255, 128, 0));
    }

    #[test]
    fn screen_rendering() {
        let mut state = TerminalState::new(5, 10, 0);
        state.process(b"ABCDE");

        let theme = TerminalTheme::default();
        let rows = state.screen(&theme);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].cells.len(), 10);
        assert_eq!(rows[0].cells[0].ch, 'A');
        assert_eq!(rows[0].cells[4].ch, 'E');
        assert_eq!(rows[0].cells[5].ch, ' ');
    }

    #[test]
    fn resize() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.resize(10, 40);

        let theme = TerminalTheme::default();
        let rows = state.screen(&theme);
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0].cells.len(), 40);
    }

    #[test]
    fn title_from_osc() {
        let mut state = TerminalState::new(24, 80, 1000);
        // OSC 2 ; title BEL — set window title
        state.process(b"\x1b]2;Nous Terminal\x07");
        assert_eq!(state.title(), "Nous Terminal");
    }

    #[test]
    fn cursor_visibility() {
        let mut state = TerminalState::new(24, 80, 1000);
        assert!(state.cursor_visible());

        state.process(b"\x1b[?25l");
        assert!(!state.cursor_visible());

        state.process(b"\x1b[?25h");
        assert!(state.cursor_visible());
    }

    #[test]
    fn alternate_screen() {
        let mut state = TerminalState::new(24, 80, 1000);
        assert!(!state.alternate_screen());

        state.process(b"\x1b[?1049h");
        assert!(state.alternate_screen());

        state.process(b"\x1b[?1049l");
        assert!(!state.alternate_screen());
    }

    #[test]
    fn clear_screen() {
        let mut state = TerminalState::new(5, 10, 0);
        state.process(b"Hello");
        assert!(state.row_text(0).starts_with("Hello"));

        state.process(b"\x1b[2J");
        let text = state.row_text(0);
        assert!(text.trim().is_empty());
    }

    #[test]
    fn inverse_style() {
        let mut state = TerminalState::new(24, 80, 1000);
        state.process(b"\x1b[7mInverse");

        let cell = state.cell(0, 0);
        assert_eq!(cell.ch, 'I');
        assert!(cell.style.inverse);
    }

    #[test]
    fn default_color_passthrough() {
        let cell = Cell::default();
        assert_eq!(cell.style.fg, Color::Default);
        assert_eq!(cell.style.bg, Color::Default);
    }
}
