use std::{fs::{self, File}, io::Write, path::{Path, PathBuf}};
use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG: &[u8] = include_bytes!("assets/default_config.yml");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadServer {
    Bonsai,
    Refresh,
    LbpSearch,
    Archive,
}

impl DownloadServer {
    pub fn get_url(&self, sha1: &[u8; 20]) -> String {
        let h = hex::encode(sha1);
        match self {
            Self::Bonsai | Self::Refresh => format!("https://lbp.lbpbonsai.com/api/v3/assets/{}/download", h),
            Self::LbpSearch => format!("https://lbparchive.zaprit.fish/{}/{}/{}", &h[..2], &h[2..4], h),
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
    pub lbp2_beta_to_retail: bool,
}

impl Config {
    pub fn read() -> Result<Self> {
        let config_path = Path::new("config.yml");
        if !config_path.exists() {
            eprintln!("WARNING: config.yml is missing, writing default config");
            let mut new_file = File::create(config_path)?;
            new_file.write_all(DEFAULT_CONFIG)?;
            
            let config: Self = serde_yaml::from_slice(DEFAULT_CONFIG)?;
            return Ok(config)
        }

        let file = File::open(config_path).context("Couldn't open config file")?;
        let config: std::result::Result<Self, serde_yaml::Error> = serde_yaml::from_reader(file);
        
        match config {
            Ok(config) => Ok(config),
            Err(_) => {
                eprintln!("WARNING: config.yml is from an old version or broken, writing default config");

                fs::copy(config_path, "config_backup.yml").context("Couldn't backup old config")?;
                eprintln!("WARNING: Old config written to config_backup.yml");

                let mut new_file = File::create(config_path)?;
                new_file.write_all(DEFAULT_CONFIG)?;

                let config: Self = serde_yaml::from_slice(DEFAULT_CONFIG)?;
                Ok(config)
            }
        }
    }
}
