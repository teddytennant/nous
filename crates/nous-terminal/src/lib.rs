//! Embedded terminal emulator for Nous.
//!
//! Provides PTY management, VT state parsing, and a platform-agnostic rendering
//! interface. Each platform (TUI, browser, desktop, WASM) supplies its own
//! rendering backend while this crate handles the terminal state machine.
//!
//! Architecture: PTY → byte stream → VT parser → render state → platform renderer

pub mod command;
pub mod completion;
pub mod editor;
pub mod history;
pub mod prompt;
mod render;
mod vt;

#[cfg(unix)]
mod pty;

#[cfg(unix)]
pub use pty::{Pty, PtySize};

pub use render::{Cell, CellStyle, Color, RenderRow, TerminalTheme};
pub use vt::TerminalState;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("pty error: {0}")]
    Pty(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("terminal not running")]
    NotRunning,
}

pub type Result<T> = std::result::Result<T, TerminalError>;

/// Terminal configuration matching the Infinite Minimalism palette.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    pub rows: u16,
    pub cols: u16,
    pub scrollback_lines: usize,
    pub theme: TerminalTheme,
    pub shell: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            scrollback_lines: 10_000,
            theme: TerminalTheme::default(),
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
        }
    }
}

/// A complete embedded terminal: PTY + VT state + rendering.
///
/// Platform renderers read the render state each frame and draw cells
/// using their native graphics pipeline.
#[cfg(unix)]
pub struct Terminal {
    pty: Pty,
    state: TerminalState,
    config: TerminalConfig,
}

#[cfg(unix)]
impl Terminal {
    /// Spawn a new terminal with the given configuration.
    pub fn spawn(config: TerminalConfig) -> Result<Self> {
        let size = PtySize {
            rows: config.rows,
            cols: config.cols,
        };
        let pty = Pty::spawn(&config.shell, &size)?;
        let state = TerminalState::new(config.rows, config.cols, config.scrollback_lines);
        Ok(Self { pty, state, config })
    }

    /// Feed bytes from the PTY into the VT parser.
    /// Call this after reading from the PTY file descriptor.
    pub fn process(&mut self, data: &[u8]) {
        self.state.process(data);
    }

    /// Write user input to the PTY (keystrokes, paste, etc.).
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.pty.write_all(data)
    }

    /// Read available output from the PTY. Non-blocking.
    pub fn try_read(&mut self) -> Result<Vec<u8>> {
        self.pty.try_read()
    }

    /// Read from PTY, process through VT parser, return dirty state.
    /// This is the main loop driver.
    pub fn tick(&mut self) -> Result<bool> {
        let data = self.try_read()?;
        if data.is_empty() {
            return Ok(false);
        }
        self.state.process(&data);
        Ok(true)
    }

    /// Resize the terminal.
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.config.rows = rows;
        self.config.cols = cols;
        self.pty.resize(&PtySize { rows, cols })?;
        self.state.resize(rows, cols);
        Ok(())
    }

    /// Get the current visible screen as renderable rows.
    pub fn screen(&self) -> Vec<RenderRow> {
        self.state.screen(&self.config.theme)
    }

    /// Get the cursor position (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        self.state.cursor_position()
    }

    /// Check if the child process is still alive.
    pub fn is_alive(&self) -> bool {
        self.pty.is_alive()
    }

    /// Get the raw PTY file descriptor for polling.
    pub fn pty_fd(&self) -> i32 {
        self.pty.master_fd()
    }

    /// Get a reference to the VT state.
    pub fn vt_state(&self) -> &TerminalState {
        &self.state
    }

    /// Get the terminal configuration.
    pub fn config(&self) -> &TerminalConfig {
        &self.config
    }

    /// Get the current title (set via OSC escape sequences).
    pub fn title(&self) -> &str {
        self.state.title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = TerminalConfig::default();
        assert_eq!(config.rows, 24);
        assert_eq!(config.cols, 80);
        assert_eq!(config.scrollback_lines, 10_000);
    }

    #[test]
    fn theme_default_is_dark() {
        let theme = TerminalTheme::default();
        assert_eq!(theme.background, Color::Rgb(0, 0, 0));
        assert_eq!(theme.foreground, Color::Rgb(224, 224, 224));
    }

    #[cfg(unix)]
    #[test]
    fn spawn_and_check_alive() {
        let config = TerminalConfig {
            shell: "/bin/sh".into(),
            ..Default::default()
        };
        let terminal = Terminal::spawn(config).unwrap();
        assert!(terminal.is_alive());
    }

    #[cfg(unix)]
    #[test]
    fn spawn_resize_and_read() {
        let config = TerminalConfig {
            shell: "/bin/sh".into(),
            rows: 10,
            cols: 40,
            ..Default::default()
        };
        let mut terminal = Terminal::spawn(config).unwrap();
        terminal.resize(20, 80).unwrap();
        assert_eq!(terminal.config().rows, 20);
        assert_eq!(terminal.config().cols, 80);
    }

    #[cfg(unix)]
    #[test]
    fn write_and_tick() {
        let config = TerminalConfig {
            shell: "/bin/sh".into(),
            ..Default::default()
        };
        let mut terminal = Terminal::spawn(config).unwrap();
        terminal.write(b"echo hello\n").unwrap();

        // Give the shell a moment to produce output
        std::thread::sleep(std::time::Duration::from_millis(100));

        let had_output = terminal.tick().unwrap();
        assert!(had_output);
    }

    #[cfg(unix)]
    #[test]
    fn screen_returns_rows() {
        let config = TerminalConfig {
            shell: "/bin/sh".into(),
            rows: 10,
            cols: 40,
            ..Default::default()
        };
        let mut terminal = Terminal::spawn(config).unwrap();

        // Wait for shell prompt
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = terminal.tick();

        let screen = terminal.screen();
        assert_eq!(screen.len(), 10);
        assert_eq!(screen[0].cells.len(), 40);
    }
}
