//! Terminal key binding mapper.
//!
//! Parses raw terminal escape sequences into `KeyEvent` values, and maps
//! those events to `EditorAction` commands that drive the `LineEditor`.
//! Supports Emacs-style bindings (Ctrl-A, Ctrl-E, etc.), arrow keys,
//! Home/End, and custom user-defined bindings.

use std::collections::HashMap;

/// A parsed key event from a terminal escape sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyEvent {
    /// A printable Unicode character.
    Char(char),
    /// Ctrl + a letter (a-z). The char is lowercase.
    Ctrl(char),
    /// Alt/Meta + a character.
    Alt(char),
    /// Enter / carriage return.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Delete key.
    Delete,
    /// Escape.
    Escape,
    /// Arrow keys.
    Up,
    Down,
    Left,
    Right,
    /// Home / End.
    Home,
    End,
    /// Page Up / Page Down.
    PageUp,
    PageDown,
    /// Shift + arrow keys (detected via extended sequences).
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    /// Ctrl + arrow keys.
    CtrlLeft,
    CtrlRight,
    CtrlUp,
    CtrlDown,
}

/// An action that can be dispatched to the `LineEditor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    /// Insert a character at the cursor.
    InsertChar(char),
    /// Submit the current line (Enter).
    Submit,
    /// Backspace (delete char before cursor).
    Backspace,
    /// Delete char at cursor.
    DeleteChar,
    /// Delete word backward (Ctrl-W).
    DeleteWordBack,
    /// Delete word forward (Alt-D).
    DeleteWordForward,
    /// Kill to end of line (Ctrl-K).
    KillToEnd,
    /// Kill to start of line (Ctrl-U).
    KillToStart,
    /// Yank (paste) from kill ring (Ctrl-Y).
    Yank,
    /// Move cursor left.
    MoveLeft,
    /// Move cursor right.
    MoveRight,
    /// Move to beginning of line.
    MoveHome,
    /// Move to end of line.
    MoveEnd,
    /// Move cursor one word left.
    MoveWordLeft,
    /// Move cursor one word right.
    MoveWordRight,
    /// Previous history entry.
    HistoryPrev,
    /// Next history entry.
    HistoryNext,
    /// Tab completion.
    TabComplete,
    /// Transpose characters (Ctrl-T).
    TransposeChars,
    /// Ctrl-D: exit on empty, delete char otherwise.
    CtrlD,
    /// Clear the screen (Ctrl-L).
    ClearScreen,
    /// No-op (ignore this key).
    Noop,
}

/// Result of parsing an escape sequence from a byte buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// A complete key event was parsed; consumed `bytes_consumed` bytes.
    Complete(KeyEvent, usize),
    /// The buffer might be an incomplete escape sequence — need more bytes.
    Incomplete,
    /// No valid sequence found — skip 1 byte.
    Invalid,
}

/// Parse a single key event from raw terminal input bytes.
///
/// Returns a `ParseResult` indicating whether a complete key was found,
/// the buffer is incomplete (might be a partial escape sequence), or
/// the byte is invalid/unknown.
pub fn parse_key(buf: &[u8]) -> ParseResult {
    if buf.is_empty() {
        return ParseResult::Incomplete;
    }

    match buf[0] {
        // Enter (CR or LF)
        b'\r' | b'\n' => ParseResult::Complete(KeyEvent::Enter, 1),

        // Tab
        b'\t' => ParseResult::Complete(KeyEvent::Tab, 1),

        // Backspace (DEL or BS)
        0x7f | 0x08 => ParseResult::Complete(KeyEvent::Backspace, 1),

        // Ctrl-A through Ctrl-Z (0x01-0x1A), excluding special cases
        b @ 0x01..=0x1a => {
            let ch = (b + b'a' - 1) as char;
            match ch {
                'i' => ParseResult::Complete(KeyEvent::Tab, 1), // Ctrl-I = Tab
                'j' | 'm' => ParseResult::Complete(KeyEvent::Enter, 1), // Ctrl-J/M = Enter
                _ => ParseResult::Complete(KeyEvent::Ctrl(ch), 1),
            }
        }

        // Escape
        0x1b => parse_escape(&buf[1..]),

        // Printable ASCII
        0x20..=0x7e => {
            let ch = buf[0] as char;
            ParseResult::Complete(KeyEvent::Char(ch), 1)
        }

        // UTF-8 multi-byte sequences
        b @ 0xc0..=0xdf => parse_utf8(buf, b, 2),
        b @ 0xe0..=0xef => parse_utf8(buf, b, 3),
        b @ 0xf0..=0xf7 => parse_utf8(buf, b, 4),

        // Anything else
        _ => ParseResult::Invalid,
    }
}

/// Parse an escape sequence (after the initial 0x1b byte).
fn parse_escape(buf: &[u8]) -> ParseResult {
    if buf.is_empty() {
        // Lone escape — might be incomplete sequence or just Esc
        // Use a heuristic: if alone, treat as Escape
        return ParseResult::Complete(KeyEvent::Escape, 1);
    }

    match buf[0] {
        // CSI sequence: ESC [
        b'[' => parse_csi(&buf[1..]),

        // SS3 sequences: ESC O (must be checked before Alt-letter)
        b'O' => parse_ss3(&buf[1..]),

        // Alt + letter: ESC <char>
        b'a'..=b'z' => {
            let ch = buf[0] as char;
            ParseResult::Complete(KeyEvent::Alt(ch), 2)
        }
        b'A'..=b'Z' => {
            let ch = buf[0] as char;
            ParseResult::Complete(KeyEvent::Alt(ch), 2)
        }

        // ESC + DEL = Alt-Backspace (treat as Alt-Backspace)
        0x7f => ParseResult::Complete(KeyEvent::Alt('\x7f'), 2),

        // Just escape
        _ => ParseResult::Complete(KeyEvent::Escape, 1),
    }
}

/// Parse a CSI sequence: ESC [ ...
fn parse_csi(buf: &[u8]) -> ParseResult {
    if buf.is_empty() {
        return ParseResult::Incomplete;
    }

    // Simple arrow keys: ESC [ A/B/C/D
    match buf[0] {
        b'A' => return ParseResult::Complete(KeyEvent::Up, 3),
        b'B' => return ParseResult::Complete(KeyEvent::Down, 3),
        b'C' => return ParseResult::Complete(KeyEvent::Right, 3),
        b'D' => return ParseResult::Complete(KeyEvent::Left, 3),
        b'H' => return ParseResult::Complete(KeyEvent::Home, 3),
        b'F' => return ParseResult::Complete(KeyEvent::End, 3),
        _ => {}
    }

    // Extended sequences: ESC [ <num> ~ or ESC [ 1 ; <mod> <letter>
    // Collect parameter bytes (digits, semicolons)
    let mut i = 0;
    while i < buf.len() && (buf[i].is_ascii_digit() || buf[i] == b';') {
        i += 1;
    }

    if i >= buf.len() {
        return ParseResult::Incomplete;
    }

    let params = &buf[..i];
    let final_byte = buf[i];
    let total_consumed = 2 + i + 1; // ESC + [ + params + final

    // Parse parameter string
    let param_str = std::str::from_utf8(params).unwrap_or("");

    match final_byte {
        b'~' => {
            // ESC [ <num> ~
            match param_str {
                "3" => ParseResult::Complete(KeyEvent::Delete, total_consumed),
                "5" => ParseResult::Complete(KeyEvent::PageUp, total_consumed),
                "6" => ParseResult::Complete(KeyEvent::PageDown, total_consumed),
                "1" | "7" => ParseResult::Complete(KeyEvent::Home, total_consumed),
                "4" | "8" => ParseResult::Complete(KeyEvent::End, total_consumed),
                _ => ParseResult::Complete(KeyEvent::Escape, total_consumed),
            }
        }
        // Modified cursor keys: ESC [ 1 ; <mod> <letter>
        b'A' | b'B' | b'C' | b'D' => {
            let parts: Vec<&str> = param_str.split(';').collect();
            let modifier = parts.get(1).and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);

            let key = match (modifier, final_byte) {
                (2, b'A') => KeyEvent::ShiftUp,
                (2, b'B') => KeyEvent::ShiftDown,
                (2, b'C') => KeyEvent::ShiftRight,
                (2, b'D') => KeyEvent::ShiftLeft,
                (5, b'A') => KeyEvent::CtrlUp,
                (5, b'B') => KeyEvent::CtrlDown,
                (5, b'C') => KeyEvent::CtrlRight,
                (5, b'D') => KeyEvent::CtrlLeft,
                _ => match final_byte {
                    b'A' => KeyEvent::Up,
                    b'B' => KeyEvent::Down,
                    b'C' => KeyEvent::Right,
                    b'D' => KeyEvent::Left,
                    _ => KeyEvent::Escape,
                },
            };
            ParseResult::Complete(key, total_consumed)
        }
        _ => ParseResult::Complete(KeyEvent::Escape, total_consumed),
    }
}

/// Parse an SS3 sequence: ESC O ...
fn parse_ss3(buf: &[u8]) -> ParseResult {
    if buf.is_empty() {
        return ParseResult::Incomplete;
    }
    match buf[0] {
        b'A' => ParseResult::Complete(KeyEvent::Up, 3),
        b'B' => ParseResult::Complete(KeyEvent::Down, 3),
        b'C' => ParseResult::Complete(KeyEvent::Right, 3),
        b'D' => ParseResult::Complete(KeyEvent::Left, 3),
        b'H' => ParseResult::Complete(KeyEvent::Home, 3),
        b'F' => ParseResult::Complete(KeyEvent::End, 3),
        _ => ParseResult::Complete(KeyEvent::Escape, 2),
    }
}

/// Parse a UTF-8 multi-byte sequence.
fn parse_utf8(buf: &[u8], _first_byte: u8, expected_len: usize) -> ParseResult {
    if buf.len() < expected_len {
        return ParseResult::Incomplete;
    }
    match std::str::from_utf8(&buf[..expected_len]) {
        Ok(s) => {
            if let Some(ch) = s.chars().next() {
                ParseResult::Complete(KeyEvent::Char(ch), expected_len)
            } else {
                ParseResult::Invalid
            }
        }
        Err(_) => ParseResult::Invalid,
    }
}

/// Maps `KeyEvent` values to `EditorAction` values.
///
/// Provides a default Emacs-style binding set and supports custom overrides.
pub struct KeyMap {
    bindings: HashMap<KeyEvent, EditorAction>,
}

impl KeyMap {
    /// Create a key map with default Emacs-style bindings.
    pub fn new() -> Self {
        let mut bindings = HashMap::new();

        // Movement
        bindings.insert(KeyEvent::Left, EditorAction::MoveLeft);
        bindings.insert(KeyEvent::Right, EditorAction::MoveRight);
        bindings.insert(KeyEvent::Home, EditorAction::MoveHome);
        bindings.insert(KeyEvent::End, EditorAction::MoveEnd);
        bindings.insert(KeyEvent::Ctrl('a'), EditorAction::MoveHome);
        bindings.insert(KeyEvent::Ctrl('e'), EditorAction::MoveEnd);
        bindings.insert(KeyEvent::Ctrl('b'), EditorAction::MoveLeft);
        bindings.insert(KeyEvent::Ctrl('f'), EditorAction::MoveRight);
        bindings.insert(KeyEvent::Alt('b'), EditorAction::MoveWordLeft);
        bindings.insert(KeyEvent::Alt('f'), EditorAction::MoveWordRight);
        bindings.insert(KeyEvent::CtrlLeft, EditorAction::MoveWordLeft);
        bindings.insert(KeyEvent::CtrlRight, EditorAction::MoveWordRight);

        // Editing
        bindings.insert(KeyEvent::Backspace, EditorAction::Backspace);
        bindings.insert(KeyEvent::Delete, EditorAction::DeleteChar);
        bindings.insert(KeyEvent::Ctrl('d'), EditorAction::CtrlD);
        bindings.insert(KeyEvent::Ctrl('w'), EditorAction::DeleteWordBack);
        bindings.insert(KeyEvent::Alt('d'), EditorAction::DeleteWordForward);
        bindings.insert(KeyEvent::Ctrl('k'), EditorAction::KillToEnd);
        bindings.insert(KeyEvent::Ctrl('u'), EditorAction::KillToStart);
        bindings.insert(KeyEvent::Ctrl('y'), EditorAction::Yank);
        bindings.insert(KeyEvent::Ctrl('t'), EditorAction::TransposeChars);

        // History
        bindings.insert(KeyEvent::Up, EditorAction::HistoryPrev);
        bindings.insert(KeyEvent::Down, EditorAction::HistoryNext);
        bindings.insert(KeyEvent::Ctrl('p'), EditorAction::HistoryPrev);
        bindings.insert(KeyEvent::Ctrl('n'), EditorAction::HistoryNext);

        // Completion
        bindings.insert(KeyEvent::Tab, EditorAction::TabComplete);

        // Submit
        bindings.insert(KeyEvent::Enter, EditorAction::Submit);

        // Screen
        bindings.insert(KeyEvent::Ctrl('l'), EditorAction::ClearScreen);

        Self { bindings }
    }

    /// Look up the action for a key event. Printable characters default
    /// to `InsertChar`; unknown keys default to `Noop`.
    pub fn lookup(&self, key: &KeyEvent) -> EditorAction {
        if let Some(action) = self.bindings.get(key) {
            return action.clone();
        }
        match key {
            KeyEvent::Char(ch) => EditorAction::InsertChar(*ch),
            _ => EditorAction::Noop,
        }
    }

    /// Override or add a binding.
    pub fn bind(&mut self, key: KeyEvent, action: EditorAction) {
        self.bindings.insert(key, action);
    }

    /// Remove a binding.
    pub fn unbind(&mut self, key: &KeyEvent) -> Option<EditorAction> {
        self.bindings.remove(key)
    }

    /// Check if a key has an explicit binding (not a default).
    pub fn has_binding(&self, key: &KeyEvent) -> bool {
        self.bindings.contains_key(key)
    }

    /// Number of explicit bindings.
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse all complete key events from a byte buffer, returning
/// the actions and the number of bytes consumed.
pub fn process_input(buf: &[u8], keymap: &KeyMap) -> (Vec<EditorAction>, usize) {
    let mut actions = Vec::new();
    let mut consumed = 0;

    while consumed < buf.len() {
        match parse_key(&buf[consumed..]) {
            ParseResult::Complete(key, n) => {
                actions.push(keymap.lookup(&key));
                consumed += n;
            }
            ParseResult::Incomplete => break,
            ParseResult::Invalid => {
                consumed += 1;
            }
        }
    }

    (actions, consumed)
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_key: simple keys ---

    #[test]
    fn parse_enter_cr() {
        assert_eq!(parse_key(b"\r"), ParseResult::Complete(KeyEvent::Enter, 1));
    }

    #[test]
    fn parse_enter_lf() {
        assert_eq!(parse_key(b"\n"), ParseResult::Complete(KeyEvent::Enter, 1));
    }

    #[test]
    fn parse_tab() {
        assert_eq!(parse_key(b"\t"), ParseResult::Complete(KeyEvent::Tab, 1));
    }

    #[test]
    fn parse_backspace_del() {
        assert_eq!(
            parse_key(&[0x7f]),
            ParseResult::Complete(KeyEvent::Backspace, 1)
        );
    }

    #[test]
    fn parse_backspace_bs() {
        assert_eq!(
            parse_key(&[0x08]),
            ParseResult::Complete(KeyEvent::Backspace, 1)
        );
    }

    #[test]
    fn parse_printable_ascii() {
        assert_eq!(
            parse_key(b"a"),
            ParseResult::Complete(KeyEvent::Char('a'), 1)
        );
        assert_eq!(
            parse_key(b"Z"),
            ParseResult::Complete(KeyEvent::Char('Z'), 1)
        );
        assert_eq!(
            parse_key(b" "),
            ParseResult::Complete(KeyEvent::Char(' '), 1)
        );
        assert_eq!(
            parse_key(b"~"),
            ParseResult::Complete(KeyEvent::Char('~'), 1)
        );
    }

    #[test]
    fn parse_empty_buffer() {
        assert_eq!(parse_key(b""), ParseResult::Incomplete);
    }

    // --- parse_key: ctrl keys ---

    #[test]
    fn parse_ctrl_a() {
        assert_eq!(
            parse_key(&[0x01]),
            ParseResult::Complete(KeyEvent::Ctrl('a'), 1)
        );
    }

    #[test]
    fn parse_ctrl_c() {
        assert_eq!(
            parse_key(&[0x03]),
            ParseResult::Complete(KeyEvent::Ctrl('c'), 1)
        );
    }

    #[test]
    fn parse_ctrl_d() {
        assert_eq!(
            parse_key(&[0x04]),
            ParseResult::Complete(KeyEvent::Ctrl('d'), 1)
        );
    }

    #[test]
    fn parse_ctrl_z() {
        assert_eq!(
            parse_key(&[0x1a]),
            ParseResult::Complete(KeyEvent::Ctrl('z'), 1)
        );
    }

    #[test]
    fn parse_ctrl_i_is_tab() {
        // Ctrl-I (0x09) is the same as Tab
        assert_eq!(parse_key(&[0x09]), ParseResult::Complete(KeyEvent::Tab, 1));
    }

    #[test]
    fn parse_ctrl_m_is_enter() {
        // Ctrl-M (0x0d) is the same as CR/Enter
        assert_eq!(
            parse_key(&[0x0d]),
            ParseResult::Complete(KeyEvent::Enter, 1)
        );
    }

    // --- parse_key: escape sequences ---

    #[test]
    fn parse_arrow_up() {
        assert_eq!(parse_key(b"\x1b[A"), ParseResult::Complete(KeyEvent::Up, 3));
    }

    #[test]
    fn parse_arrow_down() {
        assert_eq!(
            parse_key(b"\x1b[B"),
            ParseResult::Complete(KeyEvent::Down, 3)
        );
    }

    #[test]
    fn parse_arrow_right() {
        assert_eq!(
            parse_key(b"\x1b[C"),
            ParseResult::Complete(KeyEvent::Right, 3)
        );
    }

    #[test]
    fn parse_arrow_left() {
        assert_eq!(
            parse_key(b"\x1b[D"),
            ParseResult::Complete(KeyEvent::Left, 3)
        );
    }

    #[test]
    fn parse_home_csi() {
        assert_eq!(
            parse_key(b"\x1b[H"),
            ParseResult::Complete(KeyEvent::Home, 3)
        );
    }

    #[test]
    fn parse_end_csi() {
        assert_eq!(
            parse_key(b"\x1b[F"),
            ParseResult::Complete(KeyEvent::End, 3)
        );
    }

    #[test]
    fn parse_delete_key() {
        assert_eq!(
            parse_key(b"\x1b[3~"),
            ParseResult::Complete(KeyEvent::Delete, 4)
        );
    }

    #[test]
    fn parse_page_up() {
        assert_eq!(
            parse_key(b"\x1b[5~"),
            ParseResult::Complete(KeyEvent::PageUp, 4)
        );
    }

    #[test]
    fn parse_page_down() {
        assert_eq!(
            parse_key(b"\x1b[6~"),
            ParseResult::Complete(KeyEvent::PageDown, 4)
        );
    }

    #[test]
    fn parse_home_tilde() {
        assert_eq!(
            parse_key(b"\x1b[1~"),
            ParseResult::Complete(KeyEvent::Home, 4)
        );
    }

    #[test]
    fn parse_end_tilde() {
        assert_eq!(
            parse_key(b"\x1b[4~"),
            ParseResult::Complete(KeyEvent::End, 4)
        );
    }

    // --- parse_key: modified arrow keys ---

    #[test]
    fn parse_shift_right() {
        assert_eq!(
            parse_key(b"\x1b[1;2C"),
            ParseResult::Complete(KeyEvent::ShiftRight, 6)
        );
    }

    #[test]
    fn parse_shift_left() {
        assert_eq!(
            parse_key(b"\x1b[1;2D"),
            ParseResult::Complete(KeyEvent::ShiftLeft, 6)
        );
    }

    #[test]
    fn parse_ctrl_right() {
        assert_eq!(
            parse_key(b"\x1b[1;5C"),
            ParseResult::Complete(KeyEvent::CtrlRight, 6)
        );
    }

    #[test]
    fn parse_ctrl_left() {
        assert_eq!(
            parse_key(b"\x1b[1;5D"),
            ParseResult::Complete(KeyEvent::CtrlLeft, 6)
        );
    }

    #[test]
    fn parse_ctrl_up() {
        assert_eq!(
            parse_key(b"\x1b[1;5A"),
            ParseResult::Complete(KeyEvent::CtrlUp, 6)
        );
    }

    // --- parse_key: SS3 sequences ---

    #[test]
    fn parse_ss3_up() {
        assert_eq!(parse_key(b"\x1bOA"), ParseResult::Complete(KeyEvent::Up, 3));
    }

    #[test]
    fn parse_ss3_home() {
        assert_eq!(
            parse_key(b"\x1bOH"),
            ParseResult::Complete(KeyEvent::Home, 3)
        );
    }

    #[test]
    fn parse_ss3_end() {
        assert_eq!(
            parse_key(b"\x1bOF"),
            ParseResult::Complete(KeyEvent::End, 3)
        );
    }

    // --- parse_key: Alt key ---

    #[test]
    fn parse_alt_b() {
        assert_eq!(
            parse_key(b"\x1bb"),
            ParseResult::Complete(KeyEvent::Alt('b'), 2)
        );
    }

    #[test]
    fn parse_alt_f() {
        assert_eq!(
            parse_key(b"\x1bf"),
            ParseResult::Complete(KeyEvent::Alt('f'), 2)
        );
    }

    #[test]
    fn parse_alt_d() {
        assert_eq!(
            parse_key(b"\x1bd"),
            ParseResult::Complete(KeyEvent::Alt('d'), 2)
        );
    }

    #[test]
    fn parse_lone_escape() {
        assert_eq!(
            parse_key(b"\x1b"),
            ParseResult::Complete(KeyEvent::Escape, 1)
        );
    }

    // --- parse_key: UTF-8 ---

    #[test]
    fn parse_utf8_2byte() {
        // 'ñ' is 0xc3 0xb1
        let bytes = "ñ".as_bytes();
        assert_eq!(
            parse_key(bytes),
            ParseResult::Complete(KeyEvent::Char('ñ'), 2)
        );
    }

    #[test]
    fn parse_utf8_3byte() {
        // '€' is 0xe2 0x82 0xac
        let bytes = "€".as_bytes();
        assert_eq!(
            parse_key(bytes),
            ParseResult::Complete(KeyEvent::Char('€'), 3)
        );
    }

    #[test]
    fn parse_utf8_4byte() {
        // '😀' is 0xf0 0x9f 0x98 0x80
        let bytes = "😀".as_bytes();
        assert_eq!(
            parse_key(bytes),
            ParseResult::Complete(KeyEvent::Char('😀'), 4)
        );
    }

    #[test]
    fn parse_utf8_incomplete() {
        // First byte of 'ñ' without continuation
        assert_eq!(parse_key(&[0xc3]), ParseResult::Incomplete);
    }

    // --- KeyMap ---

    #[test]
    fn keymap_default_bindings() {
        let km = KeyMap::new();
        assert!(km.binding_count() > 20);
    }

    #[test]
    fn keymap_ctrl_a_is_home() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Ctrl('a')), EditorAction::MoveHome);
    }

    #[test]
    fn keymap_ctrl_e_is_end() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Ctrl('e')), EditorAction::MoveEnd);
    }

    #[test]
    fn keymap_ctrl_k_is_kill() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Ctrl('k')), EditorAction::KillToEnd);
    }

    #[test]
    fn keymap_ctrl_u_is_kill_start() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Ctrl('u')), EditorAction::KillToStart);
    }

    #[test]
    fn keymap_ctrl_y_is_yank() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Ctrl('y')), EditorAction::Yank);
    }

    #[test]
    fn keymap_ctrl_w_is_delete_word_back() {
        let km = KeyMap::new();
        assert_eq!(
            km.lookup(&KeyEvent::Ctrl('w')),
            EditorAction::DeleteWordBack
        );
    }

    #[test]
    fn keymap_alt_d_is_delete_word_forward() {
        let km = KeyMap::new();
        assert_eq!(
            km.lookup(&KeyEvent::Alt('d')),
            EditorAction::DeleteWordForward
        );
    }

    #[test]
    fn keymap_alt_b_is_word_left() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Alt('b')), EditorAction::MoveWordLeft);
    }

    #[test]
    fn keymap_alt_f_is_word_right() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Alt('f')), EditorAction::MoveWordRight);
    }

    #[test]
    fn keymap_arrows() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Up), EditorAction::HistoryPrev);
        assert_eq!(km.lookup(&KeyEvent::Down), EditorAction::HistoryNext);
        assert_eq!(km.lookup(&KeyEvent::Left), EditorAction::MoveLeft);
        assert_eq!(km.lookup(&KeyEvent::Right), EditorAction::MoveRight);
    }

    #[test]
    fn keymap_ctrl_arrows() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::CtrlLeft), EditorAction::MoveWordLeft);
        assert_eq!(km.lookup(&KeyEvent::CtrlRight), EditorAction::MoveWordRight);
    }

    #[test]
    fn keymap_enter_is_submit() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Enter), EditorAction::Submit);
    }

    #[test]
    fn keymap_tab_is_complete() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::Tab), EditorAction::TabComplete);
    }

    #[test]
    fn keymap_printable_is_insert() {
        let km = KeyMap::new();
        assert_eq!(
            km.lookup(&KeyEvent::Char('x')),
            EditorAction::InsertChar('x')
        );
        assert_eq!(
            km.lookup(&KeyEvent::Char('Z')),
            EditorAction::InsertChar('Z')
        );
    }

    #[test]
    fn keymap_unknown_is_noop() {
        let km = KeyMap::new();
        assert_eq!(km.lookup(&KeyEvent::PageUp), EditorAction::Noop);
        assert_eq!(km.lookup(&KeyEvent::ShiftUp), EditorAction::Noop);
    }

    #[test]
    fn keymap_custom_binding() {
        let mut km = KeyMap::new();
        km.bind(KeyEvent::Ctrl('x'), EditorAction::Submit);
        assert_eq!(km.lookup(&KeyEvent::Ctrl('x')), EditorAction::Submit);
    }

    #[test]
    fn keymap_unbind() {
        let mut km = KeyMap::new();
        assert!(km.has_binding(&KeyEvent::Ctrl('a')));
        let removed = km.unbind(&KeyEvent::Ctrl('a'));
        assert_eq!(removed, Some(EditorAction::MoveHome));
        assert!(!km.has_binding(&KeyEvent::Ctrl('a')));
        // Falls through to Noop since Ctrl('a') isn't a printable char
        assert_eq!(km.lookup(&KeyEvent::Ctrl('a')), EditorAction::Noop);
    }

    #[test]
    fn keymap_override_default() {
        let mut km = KeyMap::new();
        km.bind(KeyEvent::Enter, EditorAction::Noop);
        assert_eq!(km.lookup(&KeyEvent::Enter), EditorAction::Noop);
    }

    // --- process_input ---

    #[test]
    fn process_simple_text() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"abc", &km);
        assert_eq!(consumed, 3);
        assert_eq!(
            actions,
            vec![
                EditorAction::InsertChar('a'),
                EditorAction::InsertChar('b'),
                EditorAction::InsertChar('c'),
            ]
        );
    }

    #[test]
    fn process_text_with_enter() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"hi\r", &km);
        assert_eq!(consumed, 3);
        assert_eq!(
            actions,
            vec![
                EditorAction::InsertChar('h'),
                EditorAction::InsertChar('i'),
                EditorAction::Submit,
            ]
        );
    }

    #[test]
    fn process_ctrl_sequence() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"\x01\x05", &km);
        assert_eq!(consumed, 2);
        assert_eq!(actions, vec![EditorAction::MoveHome, EditorAction::MoveEnd]);
    }

    #[test]
    fn process_escape_sequence() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"\x1b[A\x1b[B", &km);
        assert_eq!(consumed, 6);
        assert_eq!(
            actions,
            vec![EditorAction::HistoryPrev, EditorAction::HistoryNext]
        );
    }

    #[test]
    fn process_empty_input() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"", &km);
        assert_eq!(consumed, 0);
        assert!(actions.is_empty());
    }

    #[test]
    fn process_mixed_input() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"a\x1b[C\x7f", &km);
        assert_eq!(consumed, 5);
        assert_eq!(
            actions,
            vec![
                EditorAction::InsertChar('a'),
                EditorAction::MoveRight,
                EditorAction::Backspace,
            ]
        );
    }

    #[test]
    fn process_utf8_input() {
        let km = KeyMap::new();
        let input = "héllo".as_bytes();
        let (actions, consumed) = process_input(input, &km);
        assert_eq!(consumed, input.len());
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], EditorAction::InsertChar('h'));
        assert_eq!(actions[1], EditorAction::InsertChar('é'));
    }

    #[test]
    fn process_alt_key_sequence() {
        let km = KeyMap::new();
        let (actions, consumed) = process_input(b"\x1bb\x1bf", &km);
        assert_eq!(consumed, 4);
        assert_eq!(
            actions,
            vec![EditorAction::MoveWordLeft, EditorAction::MoveWordRight]
        );
    }
}
