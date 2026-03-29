use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub data_dir: PathBuf,
    pub json_output: bool,
    pub verbose: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs_default(),
            json_output: false,
            verbose: false,
        }
    }
}

fn dirs_default() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".nous")
    } else {
        PathBuf::from(".nous")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = CliConfig::default();
        assert!(!config.json_output);
        assert!(!config.verbose);
    }
}
