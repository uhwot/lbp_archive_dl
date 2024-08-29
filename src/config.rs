use std::{fs::File, io::Write, path::{Path, PathBuf}};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadServer {
    Refresh,
    Archive,
}

impl DownloadServer {
    pub fn get_url(&self, sha1: &[u8; 20]) -> String {
        let h = hex::encode(sha1);
        match self {
            Self::Refresh => format!("https://lbp.littlebigrefresh.com/api/v3/assets/{}/download", h),
            Self::Archive => format!("https://archive.org/download/dry23r{}/dry{}.zip/{}%2F{}%2F{}", h.chars().next().unwrap(), &h[..2], &h[..2], &h[2..4], h),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_path: PathBuf,
    pub backup_directory: PathBuf,
    pub download_server: DownloadServer,
    pub fix_backup_version: bool,
}

impl Config {
    pub fn read() -> Self {
        let config_path = Path::new("config.yml");
        if !config_path.exists() {
            println!("config.yml is missing, writing default config");
            let mut new_file = File::create(config_path).unwrap();
            new_file.write_all(include_bytes!("assets/default_config.yml")).unwrap();
        }

        let file = File::open(config_path).expect("Couldn't open config file");
        let config: Self = serde_yml::from_reader(file).expect("Couldn't parse config");
        config
    }
}