use serde::Serialize;

pub struct Output {
    json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    pub fn print<T: Serialize + std::fmt::Display>(&self, value: &T) {
        if self.json {
            match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            }
        } else {
            println!("{value}");
        }
    }

    pub fn print_json<T: Serialize>(&self, value: &T) {
        match serde_json::to_string_pretty(value) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("serialization error: {e}"),
        }
    }

    pub fn success(&self, msg: &str) {
        if self.json {
            println!("{}", serde_json::json!({"status": "ok", "message": msg}));
        } else {
            println!("{msg}");
        }
    }

    pub fn error(&self, msg: &str) {
        if self.json {
            eprintln!("{}", serde_json::json!({"status": "error", "message": msg}));
        } else {
            eprintln!("error: {msg}");
        }
    }

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
        } else {
            // Calculate column widths
            let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
            for row in rows {
                for (i, cell) in row.iter().enumerate() {
                    if i < widths.len() {
                        widths[i] = widths[i].max(cell.len());
                    }
                }
            }

            // Print header
            let header_line: String = headers
                .iter()
                .enumerate()
                .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
                .collect::<Vec<_>>()
                .join("  ");
            println!("{header_line}");
            println!("{}", "-".repeat(header_line.len()));

            // Print rows
            for row in rows {
                let line: String = row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let w = widths.get(i).copied().unwrap_or(cell.len());
                        format!("{:<width$}", cell, width = w)
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                println!("{line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_output_success() {
        let output = Output::new(false);
        // Just verify it doesn't panic
        output.success("done");
    }

    #[test]
    fn json_output_success() {
        let output = Output::new(true);
        output.success("done");
    }

    #[test]
    fn text_output_error() {
        let output = Output::new(false);
        output.error("something failed");
    }

    #[test]
    fn json_output_error() {
        let output = Output::new(true);
        output.error("something failed");
    }

    #[test]
    fn table_text() {
        let output = Output::new(false);
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
        let output = Output::new(true);
        output.table(&["Name", "Value"], &[vec!["alice".into(), "100".into()]]);
    }
}
