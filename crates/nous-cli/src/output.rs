//! Editorial CLI output.
//!
//! All terminal styling routes through [`nous_design::palette`] — ink, ivory,
//! stone, oxblood, sage, clay. Colors auto-disable in three cases:
//!
//! 1. `--no-color` flag was passed.
//! 2. The `NO_COLOR` environment variable is set (https://no-color.org).
//! 3. Stdout is not a terminal (e.g. piped to a file or another process).
//!
//! When colors are disabled the output is *byte-clean* — zero ANSI escapes
//! anywhere, including spinners.

use std::fmt::Write as _;
use std::io::Write;
use std::time::Duration;

use crossterm::queue;
use crossterm::style::{Attribute, ResetColor, SetAttribute};
use indicatif::{ProgressBar, ProgressStyle};
use is_terminal::IsTerminal;
use nous_design::palette;
use serde::Serialize;

/// Output channel — owns color/JSON resolution and exposes the editorial
/// helper set used throughout the CLI.
#[derive(Clone)]
pub struct Output {
    json: bool,
    color: bool,
}

impl Output {
    /// Construct with explicit color/JSON modes. Useful for tests.
    pub fn new(json: bool) -> Self {
        let color = should_color(false);
        Self { json, color }
    }

    /// Construct with explicit `no_color` flag honoring `NO_COLOR` env var
    /// and stdout TTY detection.
    pub fn with_options(json: bool, no_color: bool) -> Self {
        Self {
            json,
            color: should_color(no_color),
        }
    }

    /// Whether ANSI styling is enabled on this output.
    pub fn color_enabled(&self) -> bool {
        self.color
    }

    /// Whether the output is in JSON (machine-readable) mode.
    pub fn json_mode(&self) -> bool {
        self.json
    }

    // ── primitive emission helpers ──────────────────────────────────────────

    /// Plain ivory body line. In JSON mode behaves like `print` of the value.
    pub fn print<T: Serialize + std::fmt::Display>(&self, value: &T) {
        if self.json {
            match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            }
        } else {
            println!("{}", self.style_body(&value.to_string()));
        }
    }

    /// Force-print a JSON value, regardless of mode (used for export commands).
    pub fn print_json<T: Serialize>(&self, value: &T) {
        match serde_json::to_string_pretty(value) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("serialization error: {e}"),
        }
    }

    /// Informational note — `›` in stone, then ivory message.
    pub fn info(&self, msg: &str) {
        if self.json {
            println!("{}", serde_json::json!({"status": "info", "message": msg}));
            return;
        }
        let prefix = self.color_span("\u{203a}", palette::STONE, false);
        let body = self.style_body(msg);
        println!("{prefix} {body}");
    }

    /// Success — `OK` in sage, then ivory message.
    pub fn success(&self, msg: &str) {
        if self.json {
            println!("{}", serde_json::json!({"status": "ok", "message": msg}));
            return;
        }
        let prefix = self.color_span("OK", palette::SAGE, false);
        let body = self.style_body(msg);
        println!("{prefix}  {body}");
    }

    /// Error — `!!` in oxblood (bold), then ivory message. Goes to stderr.
    pub fn error(&self, msg: &str) {
        if self.json {
            eprintln!("{}", serde_json::json!({"status": "error", "message": msg}));
            return;
        }
        let prefix = self.color_span("!!", palette::OXBLOOD, true);
        let body = self.style_body(msg);
        eprintln!("{prefix}  {body}");
    }

    /// Warning — `△` in clay, then ivory.
    pub fn warn(&self, msg: &str) {
        if self.json {
            println!("{}", serde_json::json!({"status": "warn", "message": msg}));
            return;
        }
        let prefix = self.color_span("\u{25b3}", palette::CLAY, false);
        let body = self.style_body(msg);
        println!("{prefix}  {body}");
    }

    /// `key  value` pair — meta-caps key in stone padded to 14 columns,
    /// double space, then ivory value. Mono is implicit in a terminal.
    pub fn kv(&self, key: &str, value: &str) {
        if self.json {
            println!("{}", serde_json::json!({ key: value }));
            return;
        }
        let key_caps = key.to_uppercase();
        let key_padded = format!("{:<14}", key_caps);
        let key_styled = self.color_span(&key_padded, palette::STONE, false);
        let value_styled = self.style_body(value);
        println!("{key_styled}  {value_styled}");
    }

    /// Editorial table:
    ///   - header row in meta caps (stone)
    ///   - hairline rule (`──`) under the header
    ///   - rows in ivory body, single-space gutter
    ///   - columns whose values parse cleanly as numeric are right-aligned
    ///   - no outer borders, no row separators
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) {
        if self.json {
            let entries: Vec<serde_json::Value> = rows
                .iter()
                .map(|row| {
                    let mut map = serde_json::Map::new();
                    for (i, header) in headers.iter().enumerate() {
                        map.insert(
                            header.to_string(),
                            serde_json::Value::String(row.get(i).cloned().unwrap_or_default()),
                        );
                    }
                    serde_json::Value::Object(map)
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&entries).unwrap_or_default()
            );
            return;
        }

        // Column widths — cells use display width (assume ASCII-ish for now).
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(display_width(cell));
                }
            }
        }

        // Decide right-alignment per column: every non-empty cell must parse
        // as f64 *or* match a "numeric with optional unit" shape (e.g. "10ms",
        // "0.5 ETH"). A purely empty column collapses to left-align.
        let numeric: Vec<bool> = (0..widths.len())
            .map(|col| {
                let all_cells_numeric = rows
                    .iter()
                    .map(|r| r.get(col).map(|s| s.as_str()).unwrap_or(""))
                    .filter(|s| !s.is_empty())
                    .all(is_numeric_cell);
                let any_cell = rows
                    .iter()
                    .any(|r| r.get(col).map(|s| !s.is_empty()).unwrap_or(false));
                all_cells_numeric && any_cell
            })
            .collect();

        // Header row — meta caps, stone.
        let header_caps: Vec<String> = headers.iter().map(|h| h.to_uppercase()).collect();
        let header_widths: Vec<usize> = (0..widths.len())
            .map(|i| widths[i].max(header_caps[i].len()))
            .collect();
        let mut header_line = String::new();
        for (i, h) in header_caps.iter().enumerate() {
            if i > 0 {
                header_line.push(' ');
            }
            let padded = if numeric.get(i).copied().unwrap_or(false) {
                format!("{:>width$}", h, width = header_widths[i])
            } else {
                format!("{:<width$}", h, width = header_widths[i])
            };
            let _ = write!(
                header_line,
                "{}",
                self.color_span(&padded, palette::STONE, false)
            );
        }
        println!("{header_line}");

        // Hairline rule — box-drawing horizontal in rule color.
        let total_width: usize =
            header_widths.iter().sum::<usize>() + header_widths.len().saturating_sub(1);
        let rule = "\u{2500}".repeat(total_width);
        println!("{}", self.color_span(&rule, palette::RULE, false));

        // Rows.
        for row in rows {
            let mut line = String::new();
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    line.push(' ');
                }
                let w = header_widths.get(i).copied().unwrap_or(cell.len());
                let padded = if numeric.get(i).copied().unwrap_or(false) {
                    format!("{:>width$}", cell, width = w)
                } else {
                    format!("{:<width$}", cell, width = w)
                };
                let _ = write!(line, "{}", self.style_body(&padded));
            }
            println!("{line}");
        }
    }

    /// Construct a spinner with the editorial palette.
    ///
    /// In color mode the spinner glyph is oxblood and the message is ivory,
    /// prefixed with a `…` in stone. In no-color or piped mode the spinner is
    /// a hidden no-op `ProgressBar` — calling `set_message` / `tick` is safe
    /// but writes nothing, so JSON / scripted callers see clean output.
    ///
    /// Caller is responsible for calling `.finish_and_clear()` (or
    /// `.finish_with_message()`) when done.
    pub fn spinner(&self, msg: &str) -> ProgressBar {
        if !self.color || self.json {
            return ProgressBar::hidden();
        }
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(100));
        // ANSI escapes baked into the template (we only reach here when color
        // is allowed; piped + NO_COLOR have already returned hidden bars).
        let oxblood = ansi_fg(palette::OXBLOOD);
        let stone = ansi_fg(palette::STONE);
        let ivory = ansi_fg(palette::IVORY);
        let reset = "\x1b[0m";
        let template = format!(
            "{stone}\u{2026}{reset} {oxblood}{{spinner}}{reset}  {ivory}{{msg}}{reset}",
            stone = stone,
            oxblood = oxblood,
            ivory = ivory,
            reset = reset,
        );
        let style = ProgressStyle::with_template(&template)
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            // Six dots, sweeping. Restraint: not a Fancy spinner.
            .tick_strings(&[
                "\u{2022}     ",
                " \u{2022}    ",
                "  \u{2022}   ",
                "   \u{2022}  ",
                "    \u{2022} ",
                "     \u{2022}",
            ]);
        pb.set_style(style);
        pb.set_message(msg.to_string());
        pb
    }

    /// Wrap an async future with a spinner for its duration. Returns the
    /// future's value verbatim. The spinner is cleared before the future's
    /// output is allowed to print.
    pub async fn with_spinner<F, T>(&self, msg: &str, fut: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let pb = self.spinner(msg);
        let value = fut.await;
        pb.finish_and_clear();
        value
    }

    // ── internal styling helpers ────────────────────────────────────────────

    fn color_span(&self, text: &str, c: (u8, u8, u8), bold: bool) -> String {
        if !self.color {
            return text.to_string();
        }
        let (r, g, b) = c;
        let mut out = String::with_capacity(text.len() + 24);
        if bold {
            out.push_str("\x1b[1m");
        }
        let _ = write!(out, "\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text);
        out
    }

    fn style_body(&self, text: &str) -> String {
        // Body is ivory; on no-color we emit the text verbatim, no escapes.
        if !self.color {
            return text.to_string();
        }
        self.color_span(text, palette::IVORY, false)
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Determine whether ANSI styling should be emitted on stdout.
fn should_color(no_color_flag: bool) -> bool {
    if no_color_flag {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Render an `ESC[38;2;r;g;bm` sequence as a `String`.
fn ansi_fg(c: (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2)
}

/// Approximate display width (ASCII-aware, falls back to char count).
fn display_width(s: &str) -> usize {
    s.chars().count()
}

/// Heuristic: numeric-looking cells (right-align column).
fn is_numeric_cell(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // pure number
    if trimmed.parse::<f64>().is_ok() {
        return true;
    }
    // number followed by a short unit / token: "0.5 ETH", "127ms", "3 peers"
    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let first = iter.next().unwrap_or("");
    if first.parse::<f64>().is_ok() {
        return true;
    }
    // strip trailing letter unit ("127ms", "10s")
    let digits_end = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
        .unwrap_or(trimmed.len());
    if digits_end > 0
        && trimmed[..digits_end].parse::<f64>().is_ok()
        && trimmed[digits_end..]
            .chars()
            .all(|c| c.is_ascii_alphabetic())
    {
        return true;
    }
    false
}

/// Reset stdout colors / attributes if any escapes are pending. Useful at
/// process exit. Best-effort; failures are silent.
#[allow(dead_code)]
pub(crate) fn reset_terminal() {
    let mut out = std::io::stdout();
    let _ = queue!(out, SetAttribute(Attribute::Reset), ResetColor);
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_output_success() {
        let output = Output::with_options(false, true);
        output.success("done");
    }

    #[test]
    fn json_output_success() {
        let output = Output::with_options(true, true);
        output.success("done");
    }

    #[test]
    fn text_output_error() {
        let output = Output::with_options(false, true);
        output.error("something failed");
    }

    #[test]
    fn json_output_error() {
        let output = Output::with_options(true, true);
        output.error("something failed");
    }

    #[test]
    fn table_text() {
        let output = Output::with_options(false, true);
        output.table(
            &["Name", "Value"],
            &[
                vec!["alice".into(), "100".into()],
                vec!["bob".into(), "200".into()],
            ],
        );
    }

    #[test]
    fn table_json() {
        let output = Output::with_options(true, true);
        output.table(&["Name", "Value"], &[vec!["alice".into(), "100".into()]]);
    }

    #[test]
    fn no_color_disables_color() {
        let output = Output::with_options(false, true);
        assert!(!output.color_enabled());
        // The styled output has no ANSI escapes.
        let span = output.color_span("hello", palette::OXBLOOD, true);
        assert!(!span.contains('\x1b'));
        assert_eq!(span, "hello");
    }

    #[test]
    fn color_span_emits_truecolor() {
        // Force-enable color directly to test escape generation.
        let output = Output {
            json: false,
            color: true,
        };
        let span = output.color_span("x", (0xB2, 0x3A, 0x3A), false);
        assert!(span.contains("\x1b[38;2;178;58;58m"));
        assert!(span.ends_with("\x1b[0m"));
    }

    #[test]
    fn numeric_detection() {
        assert!(is_numeric_cell("42"));
        assert!(is_numeric_cell("3.14"));
        assert!(is_numeric_cell("0.5 ETH"));
        assert!(is_numeric_cell("127ms"));
        assert!(!is_numeric_cell("alice"));
        assert!(!is_numeric_cell(""));
    }

    #[test]
    fn kv_no_color_is_ascii_clean() {
        let output = Output::with_options(false, true);
        // Smoke test; no panic, no colors.
        output.kv("did", "did:key:z123");
    }

    #[test]
    fn spinner_hidden_in_no_color() {
        let output = Output::with_options(false, true);
        let pb = output.spinner("loading");
        assert!(pb.is_hidden());
    }
}
