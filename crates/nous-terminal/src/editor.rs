//! Readline-like line editor for the Nous terminal.
//!
//! Provides cursor movement, editing operations, kill/yank, history
//! navigation, and tab completion integration. The editor is stateless
//! with respect to rendering — it manages the text buffer and cursor
//! position, and the caller reads the state to render.

use crate::command::CommandRegistry;
use crate::completion::Completer;
use crate::history::History;

/// An editing action that the caller should apply.
#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    /// The line is ready to submit (user pressed Enter).
    Submit(String),
    /// The editor buffer changed — re-render the prompt line.
    Redraw,
    /// Tab completions are available — display them.
    ShowCompletions(Vec<String>),
    /// No visible change.
    Noop,
    /// The user requested to exit (Ctrl-D on empty line).
    Exit,
    /// Clear the screen.
    ClearScreen,
}

/// A readline-like line editor.
///
/// Manages a text buffer, cursor position, kill ring, and integrates
/// with the completion and history systems.
#[derive(Debug)]
pub struct LineEditor {
    /// The current text buffer.
    buffer: Vec<char>,
    /// Cursor position within the buffer (0 = beginning).
    cursor: usize,
    /// Kill ring for Ctrl-K / Ctrl-Y.
    kill_ring: String,
    /// Saved buffer when navigating history.
    saved_buffer: Option<String>,
    /// Whether we're currently navigating history.
    in_history: bool,
    /// Number of consecutive tabs (for cycling completions).
    tab_count: u32,
}

impl LineEditor {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            cursor: 0,
            kill_ring: String::new(),
            saved_buffer: None,
            in_history: false,
            tab_count: 0,
        }
    }

    /// Current buffer content as a string.
    pub fn buffer(&self) -> String {
        self.buffer.iter().collect()
    }

    /// Current cursor position (character offset from start).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Buffer length in characters.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Set the buffer content and move cursor to end.
    pub fn set_buffer(&mut self, text: &str) {
        self.buffer = text.chars().collect();
        self.cursor = self.buffer.len();
        self.tab_count = 0;
    }

    /// Clear the buffer and reset cursor.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.kill_ring.clear();
        self.saved_buffer = None;
        self.in_history = false;
        self.tab_count = 0;
    }

    // --- Character insertion ---

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, ch: char) -> EditAction {
        self.buffer.insert(self.cursor, ch);
        self.cursor += 1;
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Insert a string at the cursor position.
    pub fn insert_str(&mut self, s: &str) -> EditAction {
        for ch in s.chars() {
            self.buffer.insert(self.cursor, ch);
            self.cursor += 1;
        }
        self.tab_count = 0;
        EditAction::Redraw
    }

    // --- Deletion ---

    /// Delete the character before the cursor (Backspace).
    pub fn backspace(&mut self) -> EditAction {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Delete the character at the cursor (Delete key).
    pub fn delete_char(&mut self) -> EditAction {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Delete the word before the cursor (Ctrl-W / Alt-Backspace).
    pub fn delete_word_back(&mut self) -> EditAction {
        if self.cursor == 0 {
            return EditAction::Noop;
        }

        let start = self.find_word_start();
        let deleted: String = self.buffer[start..self.cursor].iter().collect();
        self.kill_ring = deleted;
        self.buffer.drain(start..self.cursor);
        self.cursor = start;
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Delete the word after the cursor (Alt-D).
    pub fn delete_word_forward(&mut self) -> EditAction {
        if self.cursor >= self.buffer.len() {
            return EditAction::Noop;
        }

        let end = self.find_word_end();
        let deleted: String = self.buffer[self.cursor..end].iter().collect();
        self.kill_ring = deleted;
        self.buffer.drain(self.cursor..end);
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Kill from cursor to end of line (Ctrl-K).
    pub fn kill_to_end(&mut self) -> EditAction {
        if self.cursor >= self.buffer.len() {
            return EditAction::Noop;
        }

        let killed: String = self.buffer[self.cursor..].iter().collect();
        self.kill_ring = killed;
        self.buffer.truncate(self.cursor);
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Kill from start of line to cursor (Ctrl-U).
    pub fn kill_to_start(&mut self) -> EditAction {
        if self.cursor == 0 {
            return EditAction::Noop;
        }

        let killed: String = self.buffer[..self.cursor].iter().collect();
        self.kill_ring = killed;
        self.buffer.drain(..self.cursor);
        self.cursor = 0;
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Yank (paste) the kill ring content (Ctrl-Y).
    pub fn yank(&mut self) -> EditAction {
        if self.kill_ring.is_empty() {
            return EditAction::Noop;
        }

        self.insert_str(&self.kill_ring.clone())
    }

    // --- Cursor movement ---

    /// Move cursor left one character (Left arrow / Ctrl-B).
    pub fn move_left(&mut self) -> EditAction {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Move cursor right one character (Right arrow / Ctrl-F).
    pub fn move_right(&mut self) -> EditAction {
        if self.cursor < self.buffer.len() {
            self.cursor += 1;
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Move cursor to start of line (Home / Ctrl-A).
    pub fn move_home(&mut self) -> EditAction {
        if self.cursor > 0 {
            self.cursor = 0;
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Move cursor to end of line (End / Ctrl-E).
    pub fn move_end(&mut self) -> EditAction {
        if self.cursor < self.buffer.len() {
            self.cursor = self.buffer.len();
            self.tab_count = 0;
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Move cursor to start of previous word (Alt-B / Ctrl-Left).
    pub fn move_word_left(&mut self) -> EditAction {
        if self.cursor == 0 {
            return EditAction::Noop;
        }

        self.cursor = self.find_word_start();
        self.tab_count = 0;
        EditAction::Redraw
    }

    /// Move cursor to end of next word (Alt-F / Ctrl-Right).
    pub fn move_word_right(&mut self) -> EditAction {
        if self.cursor >= self.buffer.len() {
            return EditAction::Noop;
        }

        self.cursor = self.find_word_end();
        self.tab_count = 0;
        EditAction::Redraw
    }

    // --- Line operations ---

    /// Submit the current line (Enter).
    pub fn submit(&mut self, history: &mut History) -> EditAction {
        let line = self.buffer();
        let trimmed = line.trim();

        if !trimmed.is_empty() {
            history.push(trimmed);
        }

        self.buffer.clear();
        self.cursor = 0;
        self.saved_buffer = None;
        self.in_history = false;
        self.tab_count = 0;

        EditAction::Submit(line)
    }

    /// Handle Ctrl-D: exit on empty line, delete char otherwise.
    pub fn ctrl_d(&mut self) -> EditAction {
        if self.buffer.is_empty() {
            EditAction::Exit
        } else {
            self.delete_char()
        }
    }

    /// Transpose characters before cursor (Ctrl-T).
    ///
    /// If cursor is at end: swap the two characters before the cursor.
    /// If cursor is in middle: swap the character before cursor with the one at cursor,
    /// then advance cursor.
    pub fn transpose_chars(&mut self) -> EditAction {
        if self.buffer.len() < 2 || self.cursor == 0 {
            return EditAction::Noop;
        }

        if self.cursor == self.buffer.len() {
            // At end: swap last two characters
            let len = self.buffer.len();
            self.buffer.swap(len - 2, len - 1);
        } else {
            // In middle: swap char before cursor with char at cursor, advance
            self.buffer.swap(self.cursor - 1, self.cursor);
            self.cursor += 1;
        }

        self.tab_count = 0;
        EditAction::Redraw
    }

    // --- History navigation ---

    /// Navigate to previous history entry (Up arrow / Ctrl-P).
    pub fn history_prev(&mut self, history: &mut History) -> EditAction {
        if !self.in_history {
            self.saved_buffer = Some(self.buffer());
            self.in_history = true;
        }

        if let Some(cmd) = history.previous() {
            self.set_buffer(cmd);
            EditAction::Redraw
        } else {
            EditAction::Noop
        }
    }

    /// Navigate to next history entry (Down arrow / Ctrl-N).
    pub fn history_next(&mut self, history: &mut History) -> EditAction {
        if !self.in_history {
            return EditAction::Noop;
        }

        if let Some(cmd) = history.next_entry() {
            self.set_buffer(cmd);
            EditAction::Redraw
        } else {
            // Reached the end — restore saved buffer
            self.in_history = false;
            if let Some(saved) = self.saved_buffer.take() {
                self.set_buffer(&saved);
            } else {
                self.set_buffer("");
            }
            EditAction::Redraw
        }
    }

    // --- Tab completion ---

    /// Handle Tab key for completion.
    pub fn tab_complete(
        &mut self,
        completer: &Completer,
        registry: &CommandRegistry,
    ) -> EditAction {
        let input = self.buffer();
        self.tab_count += 1;

        if self.tab_count == 1 {
            // First tab: try to complete or show common prefix
            if let Some(completed) = completer.apply_first(&input, registry) {
                self.set_buffer(&completed);
                self.tab_count = 0; // Reset on successful completion
                return EditAction::Redraw;
            }

            // Check for multiple completions
            let completions = completer.complete(&input, registry);
            if completions.len() > 1 {
                // Try common prefix
                if let Some(prefix) = completer.common_prefix(&input, registry) {
                    self.set_buffer(&prefix);
                    return EditAction::Redraw;
                }

                // Show completion list
                let display: Vec<String> = completions.iter().map(|c| c.display.clone()).collect();
                return EditAction::ShowCompletions(display);
            }
        } else if self.tab_count >= 2 {
            // Second tab: show all completions
            let completions = completer.complete(&input, registry);
            if !completions.is_empty() {
                let display: Vec<String> = completions.iter().map(|c| c.display.clone()).collect();
                self.tab_count = 0;
                return EditAction::ShowCompletions(display);
            }
        }

        EditAction::Noop
    }

    // --- Word boundary helpers ---

    /// Find the start of the word before cursor.
    fn find_word_start(&self) -> usize {
        let mut pos = self.cursor;

        // Skip whitespace before cursor
        while pos > 0 && self.buffer[pos - 1].is_whitespace() {
            pos -= 1;
        }

        // Skip word characters
        while pos > 0 && !self.buffer[pos - 1].is_whitespace() {
            pos -= 1;
        }

        pos
    }

    /// Find the end of the word after cursor.
    fn find_word_end(&self) -> usize {
        let len = self.buffer.len();
        let mut pos = self.cursor;

        // Skip whitespace after cursor
        while pos < len && self.buffer[pos].is_whitespace() {
            pos += 1;
        }

        // Skip word characters
        while pos < len && !self.buffer[pos].is_whitespace() {
            pos += 1;
        }

        pos
    }
}

impl Default for LineEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> LineEditor {
        LineEditor::new()
    }

    fn editor_with(text: &str) -> LineEditor {
        let mut e = LineEditor::new();
        e.set_buffer(text);
        e
    }

    fn history() -> History {
        History::new(100)
    }

    // --- Basic state ---

    #[test]
    fn new_editor_is_empty() {
        let e = editor();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.buffer(), "");
    }

    #[test]
    fn set_buffer() {
        let mut e = editor();
        e.set_buffer("hello");
        assert_eq!(e.buffer(), "hello");
        assert_eq!(e.cursor(), 5);
        assert_eq!(e.len(), 5);
    }

    #[test]
    fn clear() {
        let mut e = editor_with("hello");
        e.clear();
        assert!(e.is_empty());
        assert_eq!(e.cursor(), 0);
    }

    // --- Character insertion ---

    #[test]
    fn insert_char() {
        let mut e = editor();
        assert_eq!(e.insert_char('h'), EditAction::Redraw);
        assert_eq!(e.insert_char('i'), EditAction::Redraw);
        assert_eq!(e.buffer(), "hi");
        assert_eq!(e.cursor(), 2);
    }

    #[test]
    fn insert_char_at_middle() {
        let mut e = editor_with("hllo");
        e.cursor = 1;
        e.insert_char('e');
        assert_eq!(e.buffer(), "hello");
        assert_eq!(e.cursor(), 2);
    }

    #[test]
    fn insert_str() {
        let mut e = editor();
        e.insert_str("hello");
        assert_eq!(e.buffer(), "hello");
        assert_eq!(e.cursor(), 5);
    }

    // --- Deletion ---

    #[test]
    fn backspace() {
        let mut e = editor_with("hello");
        assert_eq!(e.backspace(), EditAction::Redraw);
        assert_eq!(e.buffer(), "hell");
        assert_eq!(e.cursor(), 4);
    }

    #[test]
    fn backspace_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.backspace(), EditAction::Noop);
    }

    #[test]
    fn backspace_in_middle() {
        let mut e = editor_with("hello");
        e.cursor = 3;
        e.backspace();
        assert_eq!(e.buffer(), "helo");
        assert_eq!(e.cursor(), 2);
    }

    #[test]
    fn delete_char() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.delete_char(), EditAction::Redraw);
        assert_eq!(e.buffer(), "ello");
    }

    #[test]
    fn delete_char_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.delete_char(), EditAction::Noop);
    }

    #[test]
    fn delete_word_back() {
        let mut e = editor_with("hello world");
        e.delete_word_back();
        assert_eq!(e.buffer(), "hello ");
        assert_eq!(e.cursor(), 6);
        assert_eq!(e.kill_ring, "world");
    }

    #[test]
    fn delete_word_back_multiple_spaces() {
        let mut e = editor_with("hello   world");
        e.cursor = 8; // After "hello   "
        e.delete_word_back();
        assert_eq!(e.buffer(), "world");
        assert_eq!(e.cursor(), 0);
    }

    #[test]
    fn delete_word_back_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.delete_word_back(), EditAction::Noop);
    }

    #[test]
    fn delete_word_forward() {
        let mut e = editor_with("hello world");
        e.cursor = 0;
        e.delete_word_forward();
        assert_eq!(e.buffer(), " world");
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.kill_ring, "hello");
    }

    #[test]
    fn delete_word_forward_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.delete_word_forward(), EditAction::Noop);
    }

    // --- Kill / Yank ---

    #[test]
    fn kill_to_end() {
        let mut e = editor_with("hello world");
        e.cursor = 5;
        e.kill_to_end();
        assert_eq!(e.buffer(), "hello");
        assert_eq!(e.kill_ring, " world");
    }

    #[test]
    fn kill_to_end_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.kill_to_end(), EditAction::Noop);
    }

    #[test]
    fn kill_to_start() {
        let mut e = editor_with("hello world");
        e.cursor = 5;
        e.kill_to_start();
        assert_eq!(e.buffer(), " world");
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.kill_ring, "hello");
    }

    #[test]
    fn kill_to_start_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.kill_to_start(), EditAction::Noop);
    }

    #[test]
    fn yank() {
        let mut e = editor_with("hello world");
        e.cursor = 5;
        e.kill_to_end();
        assert_eq!(e.buffer(), "hello");

        e.yank();
        assert_eq!(e.buffer(), "hello world");
    }

    #[test]
    fn yank_empty_ring() {
        let mut e = editor();
        assert_eq!(e.yank(), EditAction::Noop);
    }

    #[test]
    fn kill_and_yank_at_different_position() {
        let mut e = editor_with("abcdef");
        e.cursor = 3;
        e.kill_to_end(); // kills "def"
        assert_eq!(e.buffer(), "abc");

        e.cursor = 0;
        e.yank(); // pastes "def" at start
        assert_eq!(e.buffer(), "defabc");
    }

    // --- Cursor movement ---

    #[test]
    fn move_left() {
        let mut e = editor_with("hello");
        assert_eq!(e.move_left(), EditAction::Redraw);
        assert_eq!(e.cursor(), 4);
    }

    #[test]
    fn move_left_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.move_left(), EditAction::Noop);
    }

    #[test]
    fn move_right() {
        let mut e = editor_with("hello");
        e.cursor = 3;
        assert_eq!(e.move_right(), EditAction::Redraw);
        assert_eq!(e.cursor(), 4);
    }

    #[test]
    fn move_right_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.move_right(), EditAction::Noop);
    }

    #[test]
    fn move_home() {
        let mut e = editor_with("hello");
        assert_eq!(e.move_home(), EditAction::Redraw);
        assert_eq!(e.cursor(), 0);
    }

    #[test]
    fn move_home_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.move_home(), EditAction::Noop);
    }

    #[test]
    fn move_end() {
        let mut e = editor_with("hello");
        e.cursor = 2;
        assert_eq!(e.move_end(), EditAction::Redraw);
        assert_eq!(e.cursor(), 5);
    }

    #[test]
    fn move_end_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.move_end(), EditAction::Noop);
    }

    #[test]
    fn move_word_left() {
        let mut e = editor_with("hello world foo");
        e.move_word_left();
        assert_eq!(e.cursor(), 12); // start of "foo"

        e.move_word_left();
        assert_eq!(e.cursor(), 6); // start of "world"

        e.move_word_left();
        assert_eq!(e.cursor(), 0); // start of "hello"
    }

    #[test]
    fn move_word_left_at_start() {
        let mut e = editor_with("hello");
        e.cursor = 0;
        assert_eq!(e.move_word_left(), EditAction::Noop);
    }

    #[test]
    fn move_word_right() {
        let mut e = editor_with("hello world foo");
        e.cursor = 0;
        e.move_word_right();
        assert_eq!(e.cursor(), 5); // end of "hello"

        e.move_word_right();
        assert_eq!(e.cursor(), 11); // end of "world"

        e.move_word_right();
        assert_eq!(e.cursor(), 15); // end of "foo"
    }

    #[test]
    fn move_word_right_at_end() {
        let mut e = editor_with("hello");
        assert_eq!(e.move_word_right(), EditAction::Noop);
    }

    // --- Line operations ---

    #[test]
    fn submit() {
        let mut e = editor_with("hello");
        let mut h = history();

        let action = e.submit(&mut h);
        assert_eq!(action, EditAction::Submit("hello".into()));
        assert!(e.is_empty());
        assert_eq!(e.cursor(), 0);
    }

    #[test]
    fn submit_empty() {
        let mut e = editor();
        let mut h = history();

        let action = e.submit(&mut h);
        assert_eq!(action, EditAction::Submit("".into()));
    }

    #[test]
    fn submit_adds_to_history() {
        let mut e = editor_with("/wallet balance");
        let mut h = history();

        e.submit(&mut h);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn ctrl_d_empty() {
        let mut e = editor();
        assert_eq!(e.ctrl_d(), EditAction::Exit);
    }

    #[test]
    fn ctrl_d_with_content() {
        let mut e = editor_with("hello");
        e.cursor = 2;
        assert_eq!(e.ctrl_d(), EditAction::Redraw); // deletes char
        assert_eq!(e.buffer(), "helo");
    }

    #[test]
    fn transpose_chars() {
        let mut e = editor_with("ab");
        e.transpose_chars();
        assert_eq!(e.buffer(), "ba");
    }

    #[test]
    fn transpose_chars_in_middle() {
        let mut e = editor_with("abcd");
        e.cursor = 2;
        e.transpose_chars();
        // Swaps char before cursor (b) with char at cursor (c), advances cursor
        assert_eq!(e.buffer(), "acbd");
        assert_eq!(e.cursor(), 3);
    }

    #[test]
    fn transpose_chars_at_start() {
        let mut e = editor_with("ab");
        e.cursor = 0;
        assert_eq!(e.transpose_chars(), EditAction::Noop);
    }

    #[test]
    fn transpose_chars_single() {
        let mut e = editor_with("a");
        assert_eq!(e.transpose_chars(), EditAction::Noop);
    }

    // --- History navigation ---

    #[test]
    fn history_navigation() {
        let mut e = editor();
        let mut h = history();

        // Add some history
        let mut temp = editor_with("/help");
        temp.submit(&mut h);
        temp.set_buffer("/wallet balance");
        temp.submit(&mut h);

        // Navigate up
        e.set_buffer("current");
        e.history_prev(&mut h);
        assert_eq!(e.buffer(), "/wallet balance");

        e.history_prev(&mut h);
        assert_eq!(e.buffer(), "/help");

        // Navigate back down
        e.history_next(&mut h);
        assert_eq!(e.buffer(), "/wallet balance");

        // Navigate past end restores saved
        e.history_next(&mut h);
        assert_eq!(e.buffer(), "current");
    }

    #[test]
    fn history_prev_empty() {
        let mut e = editor();
        let mut h = history();
        assert_eq!(e.history_prev(&mut h), EditAction::Noop);
    }

    #[test]
    fn history_next_without_prev() {
        let mut e = editor();
        let mut h = history();
        assert_eq!(e.history_next(&mut h), EditAction::Noop);
    }

    // --- Tab completion ---

    #[test]
    fn tab_complete_exact() {
        let mut e = editor_with("/wal");
        let c = Completer::new();
        let r = CommandRegistry::new();

        let action = e.tab_complete(&c, &r);
        assert_eq!(action, EditAction::Redraw);
        assert_eq!(e.buffer(), "/wallet ");
    }

    #[test]
    fn tab_complete_no_match() {
        let mut e = editor_with("/zzz");
        let c = Completer::new();
        let r = CommandRegistry::new();

        let action = e.tab_complete(&c, &r);
        assert_eq!(action, EditAction::Noop);
    }

    #[test]
    fn tab_complete_non_command() {
        let mut e = editor_with("echo");
        let c = Completer::new();
        let r = CommandRegistry::new();

        let action = e.tab_complete(&c, &r);
        assert_eq!(action, EditAction::Noop);
    }

    // --- Unicode ---

    #[test]
    fn unicode_insert() {
        let mut e = editor();
        e.insert_str("café");
        assert_eq!(e.buffer(), "café");
        assert_eq!(e.len(), 4);
    }

    #[test]
    fn unicode_cursor_movement() {
        let mut e = editor_with("日本語");
        assert_eq!(e.len(), 3);

        e.move_left();
        assert_eq!(e.cursor(), 2);

        e.backspace();
        assert_eq!(e.buffer(), "日語");
    }

    // --- Edge cases ---

    #[test]
    fn word_operations_on_single_word() {
        let mut e = editor_with("hello");
        e.delete_word_back();
        assert_eq!(e.buffer(), "");
    }

    #[test]
    fn word_operations_with_leading_space() {
        let mut e = editor_with("  hello");
        e.cursor = 2;
        e.move_word_left();
        assert_eq!(e.cursor(), 0);
    }

    #[test]
    fn multiple_kill_yank_cycle() {
        let mut e = editor_with("one two three");
        e.kill_to_end(); // cursor at end, kills nothing
        // Move to middle
        e.set_buffer("one two three");
        e.cursor = 4;
        e.kill_to_end();
        assert_eq!(e.buffer(), "one ");
        assert_eq!(e.kill_ring, "two three");

        // Yank at beginning
        e.cursor = 0;
        e.yank();
        assert_eq!(e.buffer(), "two threeone ");
    }

    #[test]
    fn insert_after_move() {
        let mut e = editor_with("hllo");
        e.cursor = 1;
        e.insert_char('e');
        assert_eq!(e.buffer(), "hello");

        e.move_end();
        e.insert_char('!');
        assert_eq!(e.buffer(), "hello!");
    }
}
