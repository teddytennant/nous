use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputField {
    pub value: String,
    pub cursor: usize,
    pub placeholder: String,
}

impl InputField {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            placeholder: placeholder.into(),
        }
    }

    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.value.len());
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn take(&mut self) -> String {
        let val = std::mem::take(&mut self.value);
        self.cursor = 0;
        val
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn display_value(&self) -> &str {
        if self.is_empty() {
            &self.placeholder
        } else {
            &self.value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_input_is_empty() {
        let input = InputField::new("type here...");
        assert!(input.is_empty());
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn insert_characters() {
        let mut input = InputField::new("");
        input.insert('h');
        input.insert('i');
        assert_eq!(input.value, "hi");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn backspace() {
        let mut input = InputField::new("");
        input.insert('a');
        input.insert('b');
        input.insert('c');
        input.backspace();
        assert_eq!(input.value, "ab");
    }

    #[test]
    fn backspace_at_start() {
        let mut input = InputField::new("");
        input.backspace(); // should not panic
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn delete() {
        let mut input = InputField::new("");
        input.insert('a');
        input.insert('b');
        input.home();
        input.delete();
        assert_eq!(input.value, "b");
    }

    #[test]
    fn cursor_movement() {
        let mut input = InputField::new("");
        input.insert('a');
        input.insert('b');
        input.insert('c');
        assert_eq!(input.cursor, 3);

        input.move_left();
        assert_eq!(input.cursor, 2);

        input.move_left();
        assert_eq!(input.cursor, 1);

        input.move_right();
        assert_eq!(input.cursor, 2);

        input.home();
        assert_eq!(input.cursor, 0);

        input.end();
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn insert_at_middle() {
        let mut input = InputField::new("");
        input.insert('a');
        input.insert('c');
        input.move_left();
        input.insert('b');
        assert_eq!(input.value, "abc");
    }

    #[test]
    fn clear() {
        let mut input = InputField::new("");
        input.insert('x');
        input.insert('y');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor, 0);
    }

    #[test]
    fn take() {
        let mut input = InputField::new("");
        input.insert('h');
        input.insert('i');
        let val = input.take();
        assert_eq!(val, "hi");
        assert!(input.is_empty());
    }

    #[test]
    fn display_shows_placeholder() {
        let input = InputField::new("type here...");
        assert_eq!(input.display_value(), "type here...");
    }

    #[test]
    fn display_shows_value() {
        let mut input = InputField::new("type here...");
        input.insert('x');
        assert_eq!(input.display_value(), "x");
    }

    #[test]
    fn unicode_support() {
        let mut input = InputField::new("");
        input.insert('a');
        input.insert('é');
        input.insert('b');
        assert_eq!(input.value, "aéb");

        input.backspace();
        assert_eq!(input.value, "aé");

        input.backspace();
        assert_eq!(input.value, "a");
    }
}
