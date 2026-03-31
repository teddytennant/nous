//! Interactive REPL that wires together the terminal subsystems.
//!
//! The REPL orchestrates: raw byte input → key parsing → editor actions →
//! command parsing → builtin execution or API dispatch. Each platform
//! (TUI, browser, desktop) drives the REPL with raw terminal bytes and
//! consumes the resulting `ReplEvent` stream to render output.
//!
//! Architecture:
//! ```text
//! raw bytes → keybind::process_input → EditorAction → LineEditor → EditAction
//!     ↓                                                     ↓
//! ReplEvent ← command::parse + execute_builtin ← Submit(line)
//! ```

use crate::command::{self, CommandRegistry, CommandStatus, ParsedCommand};
use crate::completion::Completer;
use crate::editor::{EditAction, LineEditor};
use crate::history::History;
use crate::keybind::{EditorAction, KeyMap};
use crate::prompt::{self, PromptConfig, PromptState};

/// Events emitted by the REPL for the rendering layer to consume.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplEvent {
    /// The prompt line changed — re-render it.
    /// Contains the prompt string, buffer content, and cursor position.
    Redraw {
        prompt: String,
        buffer: String,
        cursor: usize,
    },

    /// A command produced output text. The status indicates success/error.
    Output { text: String, status: CommandStatus },

    /// Tab completions should be displayed to the user.
    ShowCompletions(Vec<String>),

    /// A command needs to be dispatched to the API layer.
    /// The rendering layer should send this to the API client.
    ApiDispatch(ParsedCommand),

    /// The user requested to clear the screen.
    ClearScreen,

    /// The user requested to exit the REPL.
    Exit,
}

/// Configuration for the REPL.
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Maximum history entries.
    pub max_history: usize,
    /// Optional path for persistent history.
    pub history_file: Option<String>,
    /// Prompt display configuration.
    pub prompt_config: PromptConfig,
    /// Whether to use ANSI colors in the prompt.
    pub ansi_prompt: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            history_file: None,
            prompt_config: PromptConfig::default(),
            ansi_prompt: true,
        }
    }
}

/// The interactive REPL engine.
///
/// Owns all terminal subsystems and provides a single `process_bytes`
/// entry point. The rendering layer calls this with raw terminal input
/// and iterates the returned events to update the display.
pub struct Repl {
    editor: LineEditor,
    history: History,
    completer: Completer,
    registry: CommandRegistry,
    keymap: KeyMap,
    prompt_state: PromptState,
    config: ReplConfig,
    /// Residual bytes from incomplete escape sequences.
    input_buf: Vec<u8>,
}

impl Repl {
    /// Create a new REPL with the given configuration.
    pub fn new(config: ReplConfig) -> Self {
        let history = match &config.history_file {
            Some(path) => History::with_file(path),
            None => History::new(config.max_history),
        };

        Self {
            editor: LineEditor::new(),
            history,
            completer: Completer::new(),
            registry: CommandRegistry::new(),
            keymap: KeyMap::new(),
            prompt_state: PromptState::default(),
            config,
            input_buf: Vec::new(),
        }
    }

    /// Process raw terminal input bytes and return a list of events.
    ///
    /// This is the main entry point. Feed all bytes from the terminal
    /// (PTY read, stdin, WebSocket, etc.) into this method.
    pub fn process_bytes(&mut self, bytes: &[u8]) -> Vec<ReplEvent> {
        self.input_buf.extend_from_slice(bytes);

        let (actions, consumed) = crate::keybind::process_input(&self.input_buf, &self.keymap);
        self.input_buf.drain(..consumed);

        let mut events = Vec::new();

        for action in actions {
            let edit_result = self.dispatch_action(action);
            if let Some(event) = self.translate_edit_action(edit_result) {
                events.push(event);
            }
        }

        events
    }

    /// Get the current prompt string.
    pub fn prompt_string(&self) -> String {
        if self.config.ansi_prompt {
            prompt::render_ansi(&self.prompt_state, &self.config.prompt_config)
        } else {
            prompt::render_plain(&self.prompt_state, &self.config.prompt_config)
        }
    }

    /// Get a redraw event for the current state.
    pub fn redraw_event(&self) -> ReplEvent {
        ReplEvent::Redraw {
            prompt: self.prompt_string(),
            buffer: self.editor.buffer(),
            cursor: self.editor.cursor(),
        }
    }

    /// Update the prompt state (identity, wallet, connection, etc.).
    pub fn set_prompt_state(&mut self, state: PromptState) {
        self.prompt_state = state;
    }

    /// Update known DIDs for tab completion.
    pub fn set_known_dids(&mut self, dids: Vec<String>) {
        self.completer.set_known_dids(dids);
    }

    /// Update known channel IDs for tab completion.
    pub fn set_known_channels(&mut self, channels: Vec<String>) {
        self.completer.set_known_channels(channels);
    }

    /// Add a custom key binding.
    pub fn bind_key(&mut self, key: crate::keybind::KeyEvent, action: EditorAction) {
        self.keymap.bind(key, action);
    }

    /// Get a reference to the command registry.
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Get a reference to the history.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Save history to disk (if a file path is configured).
    pub fn save_history(&self) -> std::io::Result<()> {
        self.history.save()
    }

    /// Get the current editor buffer content.
    pub fn buffer(&self) -> String {
        self.editor.buffer()
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> usize {
        self.editor.cursor()
    }

    // ── Private ─────────────────────────────────────────────────────

    /// Dispatch a keybind `EditorAction` to the appropriate `LineEditor` method.
    fn dispatch_action(&mut self, action: EditorAction) -> EditAction {
        match action {
            EditorAction::InsertChar(ch) => self.editor.insert_char(ch),
            EditorAction::Submit => self.editor.submit(&mut self.history),
            EditorAction::Backspace => self.editor.backspace(),
            EditorAction::DeleteChar => self.editor.delete_char(),
            EditorAction::DeleteWordBack => self.editor.delete_word_back(),
            EditorAction::DeleteWordForward => self.editor.delete_word_forward(),
            EditorAction::KillToEnd => self.editor.kill_to_end(),
            EditorAction::KillToStart => self.editor.kill_to_start(),
            EditorAction::Yank => self.editor.yank(),
            EditorAction::MoveLeft => self.editor.move_left(),
            EditorAction::MoveRight => self.editor.move_right(),
            EditorAction::MoveHome => self.editor.move_home(),
            EditorAction::MoveEnd => self.editor.move_end(),
            EditorAction::MoveWordLeft => self.editor.move_word_left(),
            EditorAction::MoveWordRight => self.editor.move_word_right(),
            EditorAction::HistoryPrev => self.editor.history_prev(&mut self.history),
            EditorAction::HistoryNext => self.editor.history_next(&mut self.history),
            EditorAction::TabComplete => self.editor.tab_complete(&self.completer, &self.registry),
            EditorAction::TransposeChars => self.editor.transpose_chars(),
            EditorAction::CtrlD => self.editor.ctrl_d(),
            EditorAction::ClearScreen => EditAction::ClearScreen,
            EditorAction::Noop => EditAction::Noop,
        }
    }

    /// Convert a `LineEditor` `EditAction` into a `ReplEvent`.
    fn translate_edit_action(&mut self, action: EditAction) -> Option<ReplEvent> {
        match action {
            EditAction::Redraw => Some(self.redraw_event()),

            EditAction::Submit(line) => Some(self.handle_submit(&line)),

            EditAction::ShowCompletions(completions) => {
                Some(ReplEvent::ShowCompletions(completions))
            }

            EditAction::ClearScreen => Some(ReplEvent::ClearScreen),

            EditAction::Exit => Some(ReplEvent::Exit),

            EditAction::Noop => None,
        }
    }

    /// Handle a submitted line: parse as command or emit as output.
    fn handle_submit(&mut self, line: &str) -> ReplEvent {
        let trimmed = line.trim();

        // Empty line — just redraw
        if trimmed.is_empty() {
            return self.redraw_event();
        }

        // Try to parse as a /command
        match command::parse(line) {
            Some(cmd) => self.handle_command(cmd),
            None => {
                // Not a command — echo back as plain text
                // (In a real terminal, this would go to a shell)
                ReplEvent::Output {
                    text: format!("{trimmed}\n"),
                    status: CommandStatus::Success,
                }
            }
        }
    }

    /// Handle a parsed command: run builtin or dispatch to API.
    fn handle_command(&mut self, cmd: ParsedCommand) -> ReplEvent {
        // Check for special commands first
        match cmd.name.as_str() {
            "exit" => return ReplEvent::Exit,
            "clear" => return ReplEvent::ClearScreen,
            _ => {}
        }

        // Try builtin execution
        match command::execute_builtin(&cmd, &self.registry) {
            Some(output) => ReplEvent::Output {
                text: format!("{}\n", output.text),
                status: output.status,
            },
            None => {
                // Dispatch to API layer
                ReplEvent::ApiDispatch(cmd)
            }
        }
    }
}

impl std::fmt::Debug for Repl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repl")
            .field("buffer", &self.editor.buffer())
            .field("cursor", &self.editor.cursor())
            .field("history_len", &self.history.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repl() -> Repl {
        Repl::new(ReplConfig {
            ansi_prompt: false,
            ..Default::default()
        })
    }

    // ── Basic input ─────────────────────────────────────────────────

    #[test]
    fn type_characters() {
        let mut r = repl();
        let events = r.process_bytes(b"hello");

        assert_eq!(events.len(), 5);
        for event in &events {
            assert!(matches!(event, ReplEvent::Redraw { .. }));
        }

        assert_eq!(r.buffer(), "hello");
        assert_eq!(r.cursor(), 5);
    }

    #[test]
    fn empty_input() {
        let mut r = repl();
        let events = r.process_bytes(b"");
        assert!(events.is_empty());
    }

    #[test]
    fn backspace_removes_char() {
        let mut r = repl();
        r.process_bytes(b"abc");
        let events = r.process_bytes(&[0x7f]); // DEL = backspace
        assert_eq!(events.len(), 1);
        assert_eq!(r.buffer(), "ab");
    }

    // ── Submit ──────────────────────────────────────────────────────

    #[test]
    fn submit_plain_text() {
        let mut r = repl();
        r.process_bytes(b"echo hello");
        let events = r.process_bytes(b"\r");

        assert!(events.iter().any(|e| matches!(
            e,
            ReplEvent::Output {
                status: CommandStatus::Success,
                ..
            }
        )));
        assert_eq!(r.buffer(), ""); // buffer cleared after submit
    }

    #[test]
    fn submit_empty_line() {
        let mut r = repl();
        let events = r.process_bytes(b"\r");

        // Empty submit produces a redraw, not output
        assert!(events.iter().any(|e| matches!(e, ReplEvent::Redraw { .. })));
    }

    // ── Command handling ────────────────────────────────────────────

    #[test]
    fn help_command() {
        let mut r = repl();
        r.process_bytes(b"/help");
        let events = r.process_bytes(b"\r");

        let output = events
            .iter()
            .find(|e| matches!(e, ReplEvent::Output { .. }));
        assert!(output.is_some());

        if let Some(ReplEvent::Output { text, status }) = output {
            assert_eq!(*status, CommandStatus::Success);
            assert!(text.contains("Available commands:"));
        }
    }

    #[test]
    fn help_specific_command() {
        let mut r = repl();
        r.process_bytes(b"/help wallet");
        let events = r.process_bytes(b"\r");

        let output = events
            .iter()
            .find(|e| matches!(e, ReplEvent::Output { .. }));
        assert!(output.is_some());

        if let Some(ReplEvent::Output { text, .. }) = output {
            assert!(text.contains("balance"));
        }
    }

    #[test]
    fn unknown_command() {
        let mut r = repl();
        r.process_bytes(b"/foobar");
        let events = r.process_bytes(b"\r");

        let output = events
            .iter()
            .find(|e| matches!(e, ReplEvent::Output { .. }));
        assert!(output.is_some());

        if let Some(ReplEvent::Output { text, status }) = output {
            assert_eq!(*status, CommandStatus::NotFound);
            assert!(text.contains("unknown command"));
        }
    }

    #[test]
    fn api_dispatch_for_wallet() {
        let mut r = repl();
        r.process_bytes(b"/wallet balance");
        let events = r.process_bytes(b"\r");

        let dispatch = events
            .iter()
            .find(|e| matches!(e, ReplEvent::ApiDispatch(_)));
        assert!(dispatch.is_some());

        if let Some(ReplEvent::ApiDispatch(cmd)) = dispatch {
            assert_eq!(cmd.name, "wallet");
            assert_eq!(cmd.subcommand.as_deref(), Some("balance"));
        }
    }

    #[test]
    fn api_dispatch_for_identity() {
        let mut r = repl();
        r.process_bytes(b"/identity show");
        let events = r.process_bytes(b"\r");

        assert!(
            events
                .iter()
                .any(|e| matches!(e, ReplEvent::ApiDispatch(_)))
        );
    }

    #[test]
    fn api_dispatch_for_governance() {
        let mut r = repl();
        r.process_bytes(b"/governance list");
        let events = r.process_bytes(b"\r");

        assert!(
            events
                .iter()
                .any(|e| matches!(e, ReplEvent::ApiDispatch(_)))
        );
    }

    // ── Special commands ────────────────────────────────────────────

    #[test]
    fn exit_command() {
        let mut r = repl();
        r.process_bytes(b"/exit");
        let events = r.process_bytes(b"\r");

        assert!(events.iter().any(|e| matches!(e, ReplEvent::Exit)));
    }

    #[test]
    fn clear_command() {
        let mut r = repl();
        r.process_bytes(b"/clear");
        let events = r.process_bytes(b"\r");

        assert!(events.iter().any(|e| matches!(e, ReplEvent::ClearScreen)));
    }

    #[test]
    fn ctrl_d_on_empty() {
        let mut r = repl();
        let events = r.process_bytes(&[0x04]); // Ctrl-D

        assert!(events.iter().any(|e| matches!(e, ReplEvent::Exit)));
    }

    #[test]
    fn ctrl_d_with_content() {
        let mut r = repl();
        r.process_bytes(b"abc");
        r.process_bytes(&[0x01]); // Ctrl-A (move to start)
        let events = r.process_bytes(&[0x04]); // Ctrl-D

        // Should delete char at cursor, not exit
        assert!(!events.iter().any(|e| matches!(e, ReplEvent::Exit)));
        assert!(events.iter().any(|e| matches!(e, ReplEvent::Redraw { .. })));
        assert_eq!(r.buffer(), "bc");
    }

    #[test]
    fn ctrl_l_clear_screen() {
        let mut r = repl();
        let events = r.process_bytes(&[0x0c]); // Ctrl-L

        assert!(events.iter().any(|e| matches!(e, ReplEvent::ClearScreen)));
    }

    // ── Editing operations ──────────────────────────────────────────

    #[test]
    fn ctrl_a_moves_home() {
        let mut r = repl();
        r.process_bytes(b"hello");
        r.process_bytes(&[0x01]); // Ctrl-A

        assert_eq!(r.cursor(), 0);
    }

    #[test]
    fn ctrl_e_moves_end() {
        let mut r = repl();
        r.process_bytes(b"hello");
        r.process_bytes(&[0x01]); // Home
        r.process_bytes(&[0x05]); // Ctrl-E

        assert_eq!(r.cursor(), 5);
    }

    #[test]
    fn ctrl_k_kills_to_end() {
        let mut r = repl();
        r.process_bytes(b"hello world");
        r.process_bytes(&[0x01]); // Home
        r.process_bytes(b"\x1b[C\x1b[C\x1b[C\x1b[C\x1b[C"); // Right x5
        r.process_bytes(&[0x0b]); // Ctrl-K

        assert_eq!(r.buffer(), "hello");
    }

    #[test]
    fn ctrl_u_kills_to_start() {
        let mut r = repl();
        r.process_bytes(b"hello world");
        // Move left 6 times (cursor at position 5)
        for _ in 0..6 {
            r.process_bytes(&[0x02]); // Ctrl-B
        }
        r.process_bytes(&[0x15]); // Ctrl-U

        assert_eq!(r.buffer(), " world");
    }

    #[test]
    fn ctrl_y_yanks() {
        let mut r = repl();
        r.process_bytes(b"hello world");
        r.process_bytes(&[0x15]); // Ctrl-U (kills "hello world")
        r.process_bytes(&[0x19]); // Ctrl-Y (yanks it back)

        assert_eq!(r.buffer(), "hello world");
    }

    #[test]
    fn ctrl_w_deletes_word_back() {
        let mut r = repl();
        r.process_bytes(b"hello world");
        r.process_bytes(&[0x17]); // Ctrl-W

        assert_eq!(r.buffer(), "hello ");
    }

    #[test]
    fn ctrl_t_transposes() {
        let mut r = repl();
        r.process_bytes(b"ab");
        r.process_bytes(&[0x14]); // Ctrl-T

        assert_eq!(r.buffer(), "ba");
    }

    // ── Arrow keys ──────────────────────────────────────────────────

    #[test]
    fn arrow_left_right() {
        let mut r = repl();
        r.process_bytes(b"hello");
        assert_eq!(r.cursor(), 5);

        r.process_bytes(b"\x1b[D"); // Left
        assert_eq!(r.cursor(), 4);

        r.process_bytes(b"\x1b[D"); // Left
        assert_eq!(r.cursor(), 3);

        r.process_bytes(b"\x1b[C"); // Right
        assert_eq!(r.cursor(), 4);
    }

    #[test]
    fn home_end_keys() {
        let mut r = repl();
        r.process_bytes(b"hello");

        r.process_bytes(b"\x1b[H"); // Home
        assert_eq!(r.cursor(), 0);

        r.process_bytes(b"\x1b[F"); // End
        assert_eq!(r.cursor(), 5);
    }

    #[test]
    fn ctrl_arrows_word_movement() {
        let mut r = repl();
        r.process_bytes(b"hello world foo");

        r.process_bytes(b"\x1b[1;5D"); // Ctrl-Left
        assert_eq!(r.cursor(), 12); // start of "foo"

        r.process_bytes(b"\x1b[1;5D"); // Ctrl-Left
        assert_eq!(r.cursor(), 6); // start of "world"
    }

    // ── History ─────────────────────────────────────────────────────

    #[test]
    fn history_navigation() {
        let mut r = repl();

        // Submit two commands
        r.process_bytes(b"/help\r");
        r.process_bytes(b"/wallet balance\r");

        // Navigate up
        r.process_bytes(b"\x1b[A"); // Up
        assert_eq!(r.buffer(), "/wallet balance");

        r.process_bytes(b"\x1b[A"); // Up
        assert_eq!(r.buffer(), "/help");

        // Navigate back down
        r.process_bytes(b"\x1b[B"); // Down
        assert_eq!(r.buffer(), "/wallet balance");

        r.process_bytes(b"\x1b[B"); // Down — back to current input
        assert_eq!(r.buffer(), "");
    }

    #[test]
    fn history_preserves_current_input() {
        let mut r = repl();
        r.process_bytes(b"/help\r");

        r.process_bytes(b"typing something");
        r.process_bytes(b"\x1b[A"); // Up
        assert_eq!(r.buffer(), "/help");

        r.process_bytes(b"\x1b[B"); // Down — restores saved
        assert_eq!(r.buffer(), "typing something");
    }

    // ── Tab completion ──────────────────────────────────────────────

    #[test]
    fn tab_completes_command() {
        let mut r = repl();
        r.process_bytes(b"/wal");
        r.process_bytes(b"\t");

        assert_eq!(r.buffer(), "/wallet ");
    }

    #[test]
    fn tab_completes_subcommand() {
        let mut r = repl();
        r.process_bytes(b"/wallet bal");
        r.process_bytes(b"\t");

        assert_eq!(r.buffer(), "/wallet balance ");
    }

    #[test]
    fn tab_no_match() {
        let mut r = repl();
        r.process_bytes(b"/zzz");
        let events = r.process_bytes(b"\t");

        // Should produce no visible change
        assert!(events.is_empty() || events.iter().all(|e| matches!(e, ReplEvent::Redraw { .. })));
        assert_eq!(r.buffer(), "/zzz");
    }

    // ── Prompt ──────────────────────────────────────────────────────

    #[test]
    fn prompt_string_default() {
        let r = Repl::new(ReplConfig {
            ansi_prompt: false,
            ..Default::default()
        });
        let prompt = r.prompt_string();
        assert!(prompt.contains("nous"));
        assert!(prompt.contains("offline"));
    }

    #[test]
    fn prompt_updates_with_state() {
        let mut r = Repl::new(ReplConfig {
            ansi_prompt: false,
            ..Default::default()
        });
        r.set_prompt_state(PromptState {
            connection: prompt::ConnectionStatus::Online,
            ..Default::default()
        });
        let prompt = r.prompt_string();
        assert!(prompt.contains("online"));
    }

    #[test]
    fn ansi_prompt_contains_escape_codes() {
        let r = Repl::new(ReplConfig::default()); // ansi_prompt = true
        let prompt = r.prompt_string();
        assert!(prompt.contains("\x1b["));
    }

    // ── Known DIDs / channels ───────────────────────────────────────

    #[test]
    fn known_dids_for_completion() {
        let mut r = repl();
        r.set_known_dids(vec!["did:key:z6MkAlice".into()]);

        r.process_bytes(b"/wallet send --to did:key:z6MkA");
        r.process_bytes(b"\t");

        assert_eq!(r.buffer(), "/wallet send --to did:key:z6MkAlice ");
    }

    #[test]
    fn known_channels_for_completion() {
        let mut r = repl();
        r.set_known_channels(vec!["ch-general".into(), "ch-random".into()]);

        r.process_bytes(b"/message read --channel ch-g");
        r.process_bytes(b"\t");

        assert_eq!(r.buffer(), "/message read --channel ch-general ");
    }

    // ── Input buffer handling ───────────────────────────────────────

    #[test]
    fn incomplete_escape_buffered() {
        let mut r = repl();

        // Send partial escape sequence
        let events1 = r.process_bytes(b"\x1b");
        // Lone escape is treated as Escape key, which maps to Noop
        // depending on keymap. Check that REPL doesn't crash.
        assert!(events1.is_empty() || events1.len() <= 1);

        // Normal input still works
        r.process_bytes(b"hello");
        assert_eq!(r.buffer(), "hello");
    }

    #[test]
    fn utf8_input() {
        let mut r = repl();
        r.process_bytes("café".as_bytes());
        assert_eq!(r.buffer(), "café");
    }

    #[test]
    fn mixed_input_batch() {
        let mut r = repl();
        // Type "hi", press Enter — all in one byte batch
        let events = r.process_bytes(b"hi\r");

        // Should have: 2 Redraws (for 'h' and 'i') + 1 Output
        assert!(events.iter().any(|e| matches!(e, ReplEvent::Output { .. })));
        assert_eq!(r.buffer(), ""); // cleared after submit
    }

    // ── Debug impl ──────────────────────────────────────────────────

    #[test]
    fn debug_format() {
        let r = repl();
        let debug = format!("{r:?}");
        assert!(debug.contains("Repl"));
        assert!(debug.contains("buffer"));
        assert!(debug.contains("cursor"));
    }

    // ── Redraw event structure ──────────────────────────────────────

    #[test]
    fn redraw_event_has_correct_state() {
        let mut r = repl();
        r.process_bytes(b"test");

        let event = r.redraw_event();
        if let ReplEvent::Redraw {
            prompt,
            buffer,
            cursor,
        } = event
        {
            assert!(prompt.contains("nous"));
            assert_eq!(buffer, "test");
            assert_eq!(cursor, 4);
        } else {
            panic!("expected Redraw event");
        }
    }

    // ── Full interaction sequences ──────────────────────────────────

    #[test]
    fn full_session_help_then_wallet() {
        let mut r = repl();

        // Type and submit /help
        r.process_bytes(b"/help\r");

        // Type and submit /wallet balance (dispatches to API)
        r.process_bytes(b"/wallet balance");
        let events = r.process_bytes(b"\r");

        assert!(
            events
                .iter()
                .any(|e| matches!(e, ReplEvent::ApiDispatch(_)))
        );
    }

    #[test]
    fn edit_command_before_submit() {
        let mut r = repl();

        // Type "/wallt" (typo), backspace, type "et balance"
        r.process_bytes(b"/wallt");
        r.process_bytes(&[0x7f]); // backspace
        r.process_bytes(b"et balance");

        assert_eq!(r.buffer(), "/wallet balance");

        let events = r.process_bytes(b"\r");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ReplEvent::ApiDispatch(_)))
        );
    }

    #[test]
    fn kill_yank_in_repl() {
        let mut r = repl();
        r.process_bytes(b"hello world");

        // Ctrl-A (home), Ctrl-K (kill to end), type "goodbye ", Ctrl-Y (yank)
        r.process_bytes(&[0x01]); // home
        r.process_bytes(&[0x0b]); // kill to end
        r.process_bytes(b"goodbye ");
        r.process_bytes(&[0x19]); // yank

        assert_eq!(r.buffer(), "goodbye hello world");
    }

    // ── Save history ────────────────────────────────────────────────

    #[test]
    fn save_history_no_file() {
        let r = repl();
        assert!(r.save_history().is_ok());
    }

    #[test]
    fn save_history_with_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.txt");

        let mut r = Repl::new(ReplConfig {
            history_file: Some(path.to_string_lossy().into_owned()),
            ansi_prompt: false,
            ..Default::default()
        });

        r.process_bytes(b"/help\r");
        r.process_bytes(b"/wallet balance\r");
        r.save_history().unwrap();

        assert!(path.exists());
    }

    // ── Custom keybinding ───────────────────────────────────────────

    #[test]
    fn custom_keybinding() {
        let mut r = repl();

        // Bind Ctrl-X to Submit
        r.bind_key(crate::keybind::KeyEvent::Ctrl('x'), EditorAction::Submit);

        r.process_bytes(b"/help");
        let events = r.process_bytes(&[0x18]); // Ctrl-X

        assert!(events.iter().any(|e| matches!(e, ReplEvent::Output { .. })));
    }
}
