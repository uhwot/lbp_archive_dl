use std::{collections::BTreeMap, fs, io::{stdout, Write}};
use clap::{Parser, Subcommand};
use config::Config;
use icon::make_icon;
use reqwest::ClientBuilder;
use sha1::{Digest, Sha1};

mod resource_parse;
mod resource_dl;
mod serializers;
mod xxtea;
mod labels;
mod db;
mod config;
mod icon;
mod gtf_texture;

use resource_dl::get_resource;
use serializers::lbp::make_slotlist;
use serializers::lbp::make_savearchive;
use serializers::ps3::make_sfo;
use serializers::ps3::make_pfd;
use db::{get_slot_info, GameVersion};
use resource_parse::{ResrcDescriptor, ResrcId, ResrcMethod};

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
    }
}

async fn dl_as_backup(level_id: i64, config: Config) {
    let slot_info = get_slot_info(level_id, &config.database_path);

    println!("Level found!");
    println!("Name: {}", &slot_info.name);
    println!("Creator: {}", &slot_info.np_handle);
    println!("Game: {}", slot_info.game.get_short_title());

    let mut client = ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()
        .unwrap();

    // please note:
    // fat entries NEED to be sorted by hash in the SaveArchive,
    // so we store all hashes in a BTreeSet to have them automatically sorted
    let mut hashes = BTreeMap::new();

    print!("Downloading resources");
    stdout().flush().unwrap();

    let mut icon_sha1 = None;

    let mut dl_count = 0;
    let mut fail_count = 0;
    get_resource(&slot_info.root_level, &mut client, &mut hashes, &mut dl_count, &mut fail_count, &config.download_server).await;
    if let ResrcDescriptor::Sha1(icon_hash) = slot_info.icon {
        get_resource(&icon_hash, &mut client, &mut hashes, &mut dl_count, &mut fail_count, &config.download_server).await;
        icon_sha1 = Some(icon_hash);
    }

    if hashes.get(&slot_info.root_level).unwrap().is_none() {
        panic!("rootLevel is missing from the archive, rip");
    }

    println!("Done!");
    println!("{dl_count} resources downloaded, {fail_count} failed");

    let root_resrc = hashes.get(&slot_info.root_level).unwrap().as_deref().unwrap();
    let root_resrc = ResrcId::new(root_resrc, false);

    let mut revision = if let ResrcMethod::Binary { revision, .. } = root_resrc.method {
        revision
    } else {
        panic!("rootLevel uses non-binary serialization method, is this corrupted?");
    };

    let mut gameversion = revision.get_gameversion();
    if slot_info.game != gameversion {
        println!(
            "WARNING: This is a {} level in {} format",
            slot_info.game.get_short_title(),
            gameversion.get_short_title(),
        );
        if config.fix_backup_version {
            println!("WARNING: Writing {} backup", gameversion.get_short_title());
        } else {
            gameversion = slot_info.game;
            revision = gameversion.get_latest_revision();
            println!("WARNING: Writing {} backup anyways, you should backport this level!", gameversion.get_short_title());
        }
    }

    let slot_id_str = hex::encode_upper(u32::to_be_bytes(level_id as u32));
    let bkp_name = match slot_info.is_adventure_planet {
        false => format!("{}LEVEL{}", gameversion.get_titleid(), slot_id_str),
        true => format!("{}ADVLBP3AAZ{}", gameversion.get_titleid(), slot_id_str),
    };
    let bkp_path = config.backup_directory.join(&bkp_name);
    fs::create_dir_all(&bkp_path).unwrap();

    let slt = make_slotlist(&revision, &slot_info);
    let slt_hash = Sha1::digest(&slt).into();
    hashes.insert(slt_hash, Some(slt));

    make_icon(&bkp_path, icon_sha1, &mut hashes);

    make_savearchive(&revision, slt_hash, hashes, &bkp_path);
    let sfo = make_sfo(&slot_info, &bkp_name, &bkp_path, &gameversion);

    let pfd_version = match gameversion {
        GameVersion::Lbp3 => 4,
        _ => 3,
    };
    make_pfd(pfd_version, sfo, &bkp_path);

    println!("Backup written to {bkp_name}");
}

#[tokio::main]
async fn main() {
    let config = Config::read();

    let cli = Cli::parse();

    match cli.command {
        Commands::Bkp { level_id } => dl_as_backup(level_id, config).await,
    }
}