use std::{fs, io::{stdout, Write}};
use clap::{Parser, Subcommand};
use config::Config;
use icon::make_icon;
use sha1::{Digest, Sha1};
use anyhow::{anyhow, Result};

mod resource_parse;
mod resource_dl;
mod serializers;
mod xxtea;
mod labels;
mod db;
mod config;
mod icon;
mod gtf_texture;

use serializers::lbp::{make_slotlist, make_savearchive};
use serializers::ps3::{make_sfo, make_pfd};
use db::{get_slot_info, GameVersion};
use resource_parse::{ResrcDescriptor, ResrcData, ResrcMethod};
use crate::resource_dl::{download_level, DownloadResult};

static USER_AGENT: &str = concat!(
    "lbp_archive_dl/", env!("CARGO_PKG_VERSION"),
);

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download level and save as level backup
    Bkp {
        /// Level ID from database
        level_id: i64,
        /// Force LBP3 backup
        #[arg(short, long)]
        lbp3: bool,
    }
}

async fn dl_as_backup(level_id: i64, config: Config, force_lbp3: bool) -> Result<()> {
    let slot_info = get_slot_info(level_id, &config.database_path)?;

    println!("Level found!");
    println!("Name: {}", &slot_info.name);
    println!("Creator: {}", &slot_info.np_handle);
    println!("Game: {}", slot_info.game.get_short_title());

    let mut max_parallel_downloads = config.max_parallel_downloads;
    if max_parallel_downloads > 10 {
        eprintln!("WARNING: max_parallel_downloads is too high, reverting to 10");
        max_parallel_downloads = 10;
    } else if max_parallel_downloads == 0 {
        return Err(anyhow!("max_parallel_downloads cannot be set to zero"));
    }

    print!("Downloading resources");
    stdout().flush()?;

    let mut icon_sha1 = None;
    if let ResrcDescriptor::Sha1(icon_hash) = slot_info.icon {
        icon_sha1 = Some(icon_hash);
    }

    let DownloadResult {
        mut resources,
        success_count: dl_count,
        error_count: fail_count,
    } = download_level(
        slot_info.root_level,
        icon_sha1,
        config.download_server,
        max_parallel_downloads,
    ).await?;

    println!();

    let root_resrc = resources.get(&slot_info.root_level)
        .ok_or(anyhow!("rootLevel is missing from the archive, rip"))?;

    println!("Done!");
    println!("{dl_count} resources downloaded, {fail_count} failed");

    let root_resrc = ResrcData::new(root_resrc, false)?;

    let mut revision = match root_resrc.method {
        ResrcMethod::Binary { revision, .. } => revision,
        _ => return Err(anyhow!("rootLevel uses non-binary serialization method, is this corrupted?"))
    };

    let mut gameversion = revision.get_gameversion();
    if force_lbp3 {
        if gameversion != GameVersion::Lbp3 {
            eprintln!("WARNING: Writing LBP3 backup");
            gameversion = GameVersion::Lbp3;
            revision = gameversion.get_latest_revision();
        }
    } else if slot_info.game != gameversion {
        eprintln!(
            "WARNING: This is a {} level in {} format",
            slot_info.game.get_short_title(),
            gameversion.get_short_title(),
        );
        if config.fix_backup_version {
            eprintln!("WARNING: Writing {} backup", gameversion.get_short_title());
        } else {
            eprintln!("WARNING: Writing {} backup anyways, you should backport this level!", gameversion.get_short_title());
            gameversion = slot_info.game;
            revision = gameversion.get_latest_revision();
        }
    }

    let slot_id_str = hex::encode_upper(u32::to_be_bytes(level_id as u32));
    let bkp_name = match slot_info.is_adventure_planet {
        false => format!("{}LEVEL{}", gameversion.get_titleid(), slot_id_str),
        true => format!("{}ADVLBP3AAZ{}", gameversion.get_titleid(), slot_id_str),
    };
    let bkp_path = config.backup_directory.join(&bkp_name);
    fs::create_dir_all(&bkp_path)?;

    let slt = make_slotlist(&revision, &slot_info)?;
    let slt_hash = Sha1::digest(&slt).into();
    resources.insert(slt_hash, slt);

    make_icon(&bkp_path, icon_sha1, &mut resources)?;

    make_savearchive(&revision, slt_hash, resources, &bkp_path)?;
    let sfo = make_sfo(&slot_info, &bkp_name, &bkp_path, &gameversion)?;

    let pfd_version = match gameversion {
        GameVersion::Lbp3 => 4,
        _ => 3,
    };
    make_pfd(pfd_version, sfo, &bkp_path)?;

    println!("Backup written to {bkp_name}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::read()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Bkp { level_id, lbp3 } => {
            let force_lbp3 = lbp3 || config.force_lbp3_backups;
            dl_as_backup(level_id, config, force_lbp3).await?
        },
    }

    Ok(())
}