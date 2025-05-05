use std::{fs::File, io::Write, path::{Path, PathBuf}};
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadServer {
    Bonsai,
    Refresh,
    Archive,
}

impl DownloadServer {
    pub fn get_url(&self, sha1: &[u8; 20]) -> String {
        let h = hex::encode(sha1);
        match self {
            Self::Bonsai | Self::Refresh => format!("https://lbp.littlebigrefresh.com/api/v3/assets/{}/download", h),
            Self::Archive => format!("https://archive.org/download/dry23r{}/dry{}.zip/{}%2F{}%2F{}", h.chars().next().unwrap(), &h[..2], &h[..2], &h[2..4], h),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub backup_directory: PathBuf,
    pub download_server: DownloadServer,
    pub max_parallel_downloads: usize,
    pub fix_backup_version: bool,
    pub force_lbp3_backups: bool,
}

impl Config {
    pub fn read() -> Result<Self> {
        let config_path = Path::new("config.yml");
        if !config_path.exists() {
            println!("config.yml is missing, writing default config");
            let mut new_file = File::create(config_path)?;
            new_file.write_all(include_bytes!("assets/default_config.yml"))?;
        }

        let file = File::open(config_path).context("Couldn't open config file")?;
        let config: Self = serde_yaml::from_reader(file).context("Couldn't parse config")?;
        Ok(config)
    }
}