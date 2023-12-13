use std::{fs::File, io::Read, path::Path};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DoDockerConfig {
    pub token: String,
    // pub ssh_pubkey: String,
    pub ssh_prikey: String,
    pub shell_file: String,
    pub ssh_key_ids: Vec<usize>,
}

impl DoDockerConfig {
    pub fn read_from_file(file_path_str: &str) -> anyhow::Result<Self> {
        let content = read_file(file_path_str)?;
        Ok(serde_json::from_str(&content)?)
    }
}

pub fn read_file(file_path_str: &str) -> anyhow::Result<String> {
    let file_path = Path::new(file_path_str);
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
