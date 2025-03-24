use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub refresh_rate: u64, // ms
    pub password: String,
    pub password_length: usize,
    pub port: u16,
    pub start_tray: bool,
}

impl Options {
    fn load_from_file(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        let options = serde_json::from_reader(file)?;
        Ok(options)
    }

    fn save_to_file(&self, path: &str) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    fn default_path() -> Result<String> {
        let mut path = std::env::current_exe()?;
        path.pop();
        path.push("options.json");
        Ok(path.to_string_lossy().to_string())
    }

    fn default() -> Self {
        Self {
            refresh_rate: 1000,
            password: "".to_string(),
            password_length: 12,
            port: 9000,
            start_tray: false,
        }
    }

    pub fn load_or_default() -> Result<Self> {
        let path = Self::default_path()?;
        if let Ok(options) = Self::load_from_file(&path) {
            Ok(options)
        } else {
            let options = Self::default();
            options.save_to_file(&path)?;
            Ok(options)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::default_path()?;
        self.save_to_file(&path)
    }
}
