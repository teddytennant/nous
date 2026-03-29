//! Terminal emulator panel for the Nous TUI.
//!
//! Embeds a `nous_terminal::Terminal` inside a ratatui widget, converting
//! the platform-agnostic render state into ratatui buffer cells each frame.

use nous_terminal::{CellStyle as TermCellStyle, Color as TermColor, Terminal, TerminalConfig};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RatColor, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};

use crate::theme::Theme;

/// Convert a `nous_terminal::Color` to a `ratatui::style::Color`.
fn convert_color(color: TermColor, default: RatColor) -> RatColor {
    match color {
        TermColor::Default => default,
        TermColor::Rgb(r, g, b) => RatColor::Rgb(r, g, b),
        TermColor::Indexed(idx) => RatColor::Indexed(idx),
    }
}

/// Convert a `nous_terminal::CellStyle` to a `ratatui::style::Style`.
fn convert_style(style: &TermCellStyle) -> Style {
    let (fg, bg) = if style.inverse {
        (
            convert_color(style.bg, Theme::BG),
            convert_color(style.fg, Theme::FG),
        )
    } else {
        (
            convert_color(style.fg, Theme::FG),
            convert_color(style.bg, Theme::BG),
        )
    };

    let mut rat_style = Style::default().fg(fg).bg(bg);

    if style.bold {
        rat_style = rat_style.add_modifier(Modifier::BOLD);
    }
    if style.italic {
        rat_style = rat_style.add_modifier(Modifier::ITALIC);
    }
    if style.underline {
        rat_style = rat_style.add_modifier(Modifier::UNDERLINED);
    }
    if style.strikethrough {
        rat_style = rat_style.add_modifier(Modifier::CROSSED_OUT);
    }

    rat_style
}

/// An embedded terminal emulator panel for the ratatui TUI.
///
/// Spawns a PTY, drives the VT state machine via `tick()`, and renders
/// the terminal contents into a ratatui `Buffer` each frame.
pub struct TerminalPanel {
    terminal: Terminal,
    /// Whether the child process has exited.
    exited: bool,
}

impl TerminalPanel {
    /// Create a new terminal panel, spawning a PTY with the given dimensions.
    ///
    /// `rows` and `cols` refer to the inner content area (excluding borders).
    pub fn new(rows: u16, cols: u16) -> nous_terminal::Result<Self> {
        let config = TerminalConfig {
            rows,
            cols,
            ..Default::default()
        };
        let terminal = Terminal::spawn(config)?;
        Ok(Self {
            terminal,
            exited: false,
        })
    }

    /// Create a terminal panel with a custom shell command.
    pub fn with_shell(rows: u16, cols: u16, shell: String) -> nous_terminal::Result<Self> {
        let config = TerminalConfig {
            rows,
            cols,
            shell,
            ..Default::default()
        };
        let terminal = Terminal::spawn(config)?;
        Ok(Self {
            terminal,
            exited: false,
        })
    }

    /// Drive the terminal: read PTY output and update VT state.
    ///
    /// Returns `true` if new output was processed (screen may be dirty).
    /// Returns `false` if there was nothing to read or the child has exited.
    pub fn tick(&mut self) -> bool {
        if self.exited {
            return false;
        }
        if !self.terminal.is_alive() {
            self.exited = true;
            return false;
        }
        match self.terminal.tick() {
            Ok(dirty) => dirty,
            Err(_) => {
                self.exited = true;
                false
            }
        }
    }

    /// Write user input (keystrokes, paste data) to the PTY.
    pub fn write_input(&mut self, data: &[u8]) -> nous_terminal::Result<()> {
        if self.exited {
            return Err(nous_terminal::TerminalError::NotRunning);
        }
        self.terminal.write(data)
    }

    /// Resize the embedded terminal to fit a new area.
    ///
    /// `rows` and `cols` should be the inner content dimensions (excluding
    /// the border).
    pub fn resize(&mut self, rows: u16, cols: u16) -> nous_terminal::Result<()> {
        if self.exited {
            return Ok(());
        }
        self.terminal.resize(rows, cols)
    }

    /// Whether the child process has exited.
    pub fn is_exited(&self) -> bool {
        self.exited
    }

    /// Get the terminal title (set via OSC escape sequences).
    pub fn title(&self) -> &str {
        self.terminal.title()
    }

    /// Get the cursor position relative to the terminal content area.
    pub fn cursor_position(&self) -> (u16, u16) {
        self.terminal.cursor_position()
    }

    /// Get the raw PTY file descriptor for external polling (e.g. with mio/tokio).
    pub fn pty_fd(&self) -> i32 {
        self.terminal.pty_fd()
    }

    /// Render the terminal panel into a ratatui frame area.
    ///
    /// This is the primary rendering method. It draws a bordered block and
    /// fills it with the current terminal screen contents, including cursor.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Draw the outer block with border
        let title_text = self.panel_title();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::border())
            .title(ratatui::text::Span::styled(
                format!(" {title_text} "),
                Theme::title(),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        // Fill the inner area with the terminal background
        for y in inner.y..inner.bottom() {
            for x in inner.x..inner.right() {
                if let Some(cell) = buf.cell_mut(ratatui::layout::Position { x, y }) {
                    cell.set_style(Style::default().fg(Theme::FG).bg(Theme::BG));
                    cell.set_char(' ');
                }
            }
        }

        // Render terminal screen contents
        let screen = self.terminal.screen();
        let (cursor_row, cursor_col) = self.terminal.cursor_position();
        let cursor_visible = self.terminal.vt_state().cursor_visible();

        for (row_idx, render_row) in screen.iter().enumerate() {
            let y = inner.y + row_idx as u16;
            if y >= inner.bottom() {
                break;
            }

            for (col_idx, term_cell) in render_row.cells.iter().enumerate() {
                let x = inner.x + col_idx as u16;
                if x >= inner.right() {
                    break;
                }

                let style = convert_style(&term_cell.style);

                if let Some(buf_cell) = buf.cell_mut(ratatui::layout::Position { x, y }) {
                    buf_cell.set_char(term_cell.ch);
                    buf_cell.set_style(style);
                }
            }
        }

        // Draw cursor
        if cursor_visible && !self.exited {
            let cursor_x = inner.x + cursor_col;
            let cursor_y = inner.y + cursor_row;

            if cursor_x < inner.right()
                && cursor_y < inner.bottom()
                && let Some(buf_cell) = buf.cell_mut(ratatui::layout::Position {
                    x: cursor_x,
                    y: cursor_y,
                })
            {
                // Gold cursor from the Infinite Minimalism palette
                buf_cell.set_style(
                    Style::default()
                        .fg(Theme::BG)
                        .bg(Theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                );
            }
        }

        // If the child has exited, show a status indicator
        if self.exited {
            let msg = " [process exited] ";
            let msg_x = inner.x + inner.width.saturating_sub(msg.len() as u16) / 2;
            let msg_y = inner.y + inner.height.saturating_sub(1) / 2;

            if msg_y < inner.bottom() {
                for (i, ch) in msg.chars().enumerate() {
                    let x = msg_x + i as u16;
                    if x < inner.right()
                        && let Some(buf_cell) =
                            buf.cell_mut(ratatui::layout::Position { x, y: msg_y })
                    {
                        buf_cell.set_char(ch);
                        buf_cell.set_style(Theme::dim());
                    }
                }
            }
        }
    }

    /// Build the panel title string.
    fn panel_title(&self) -> String {
        let title = self.terminal.title();
        if title.is_empty() {
            if self.exited {
                "Terminal (exited)".to_string()
            } else {
                "Terminal".to_string()
            }
        } else if self.exited {
            format!("{title} (exited)")
        } else {
            title.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_default_color() {
        assert_eq!(
            convert_color(TermColor::Default, RatColor::White),
            RatColor::White
        );
    }

    #[test]
    fn convert_rgb_color() {
        assert_eq!(
            convert_color(TermColor::Rgb(255, 128, 0), RatColor::Reset),
            RatColor::Rgb(255, 128, 0)
        );
    }

    #[test]
    fn convert_indexed_color() {
        assert_eq!(
            convert_color(TermColor::Indexed(5), RatColor::Reset),
            RatColor::Indexed(5)
        );
    }

    #[test]
    fn convert_plain_style() {
        let style = TermCellStyle::default();
        let rat = convert_style(&style);
        assert_eq!(rat.fg, Some(Theme::FG));
        assert_eq!(rat.bg, Some(Theme::BG));
    }

    #[test]
    fn convert_bold_style() {
        let style = TermCellStyle {
            bold: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        assert!(rat.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn convert_italic_style() {
        let style = TermCellStyle {
            italic: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        assert!(rat.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn convert_underline_style() {
        let style = TermCellStyle {
            underline: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        assert!(rat.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn convert_strikethrough_style() {
        let style = TermCellStyle {
            strikethrough: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        assert!(rat.add_modifier.contains(Modifier::CROSSED_OUT));
    }

    #[test]
    fn convert_inverse_style() {
        let style = TermCellStyle {
            fg: TermColor::Rgb(200, 200, 200),
            bg: TermColor::Rgb(10, 10, 10),
            inverse: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        // Inverse swaps fg and bg
        assert_eq!(rat.fg, Some(RatColor::Rgb(10, 10, 10)));
        assert_eq!(rat.bg, Some(RatColor::Rgb(200, 200, 200)));
    }

    #[test]
    fn convert_combined_modifiers() {
        let style = TermCellStyle {
            bold: true,
            italic: true,
            underline: true,
            strikethrough: true,
            ..Default::default()
        };
        let rat = convert_style(&style);
        assert!(rat.add_modifier.contains(Modifier::BOLD));
        assert!(rat.add_modifier.contains(Modifier::ITALIC));
        assert!(rat.add_modifier.contains(Modifier::UNDERLINED));
        assert!(rat.add_modifier.contains(Modifier::CROSSED_OUT));
    }

    #[test]
    fn spawn_terminal_panel() {
        let panel = TerminalPanel::new(10, 40).unwrap();
        assert!(!panel.is_exited());
    }

    #[test]
    fn spawn_with_shell() {
        let panel = TerminalPanel::with_shell(10, 40, "/bin/sh".into()).unwrap();
        assert!(!panel.is_exited());
    }

    #[test]
    fn tick_reads_output() {
        let mut panel = TerminalPanel::new(10, 40).unwrap();
        panel.write_input(b"echo hello\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let dirty = panel.tick();
        assert!(dirty);
    }

    #[test]
    fn write_input_to_pty() {
        let mut panel = TerminalPanel::new(10, 40).unwrap();
        panel.write_input(b"echo test\n").unwrap();
    }

    #[test]
    fn resize_panel() {
        let mut panel = TerminalPanel::new(10, 40).unwrap();
        panel.resize(20, 80).unwrap();
    }

    #[test]
    fn render_into_buffer() {
        let mut panel = TerminalPanel::new(10, 40).unwrap();
        panel.write_input(b"echo render_test\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        panel.tick();

        let area = Rect::new(0, 0, 42, 12); // +2 for borders
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);

        // The border should be present — top-left corner is a border character
        let top_left = buf.cell(ratatui::layout::Position { x: 0, y: 0 }).unwrap();
        assert_ne!(top_left.symbol(), " ");
    }

    #[test]
    fn render_empty_terminal() {
        let panel = TerminalPanel::new(5, 20).unwrap();
        let area = Rect::new(0, 0, 22, 7);
        let mut buf = Buffer::empty(area);
        panel.render(area, &mut buf);

        // Should not panic, should fill buffer
        let inner_cell = buf.cell(ratatui::layout::Position { x: 1, y: 1 }).unwrap();
        // Inner area should have been written
        assert!(inner_cell.symbol().len() > 0);
    }

    #[test]
    fn title_default_is_terminal() {
        let panel = TerminalPanel::new(5, 20).unwrap();
        assert_eq!(panel.panel_title(), "Terminal");
    }

    #[test]
    fn cursor_position_starts_at_origin() {
        let panel = TerminalPanel::new(10, 40).unwrap();
        let (row, col) = panel.cursor_position();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn exited_panel_rejects_input() {
        let mut panel = TerminalPanel::new(5, 20).unwrap();
        panel.write_input(b"exit\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Tick until exited
        for _ in 0..10 {
            panel.tick();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if panel.is_exited() {
            let result = panel.write_input(b"should fail\n");
            assert!(result.is_err());
        }
    }
}
