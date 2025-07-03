use std::path::Path;

use bitvec::{order::Lsb0, view::BitView};
use sqlite::State;
use anyhow::{anyhow, Result};

use crate::{labels::LABEL_LAMS_KEY_IDS, resource_parse::ResrcRevision, ResrcDescriptor};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum GameVersion {
    Lbp1,
    Lbp2,
    Lbp3,
}

impl GameVersion {
    pub fn get_title(&self) -> &'static str {
        match self {
            Self::Lbp1 => "LittleBigPlanet™",
            Self::Lbp2 => "LittleBigPlanet™2",
            Self::Lbp3 => "LittleBigPlanet™3",
        }
    }
    pub fn get_short_title(&self) -> &'static str {
        match self {
            Self::Lbp1 => "LBP1",
            Self::Lbp2 => "LBP2",
            Self::Lbp3 => "LBP3",
        }
    }
    pub fn get_titleid(&self) -> &'static str {
        match self {
            Self::Lbp1 => "BCES00141",
            Self::Lbp2 => "BCES00850",
            Self::Lbp3 => "BCES01663",
        }
    }
    pub fn get_latest_revision(&self) -> ResrcRevision {
        match self {
            Self::Lbp1 => ResrcRevision {
                head: 0x272,
                branch_id: 0x4c44,
                branch_revision: 0x17,
            },
            Self::Lbp2 => ResrcRevision {
                head: 0x3f8,
                branch_id: 0x0,
                branch_revision: 0x0,
            },
            Self::Lbp3 => ResrcRevision {
                head: 0x21803f9,
                branch_id: 0x0,
                branch_revision: 0x0,
            },
        }
    }
}

#[derive(Debug)]
pub enum LevelType {
    Cooperative,
    Versus,
    Cutscene,
}

#[derive(Debug)]
pub struct SlotInfo {
    pub name: String,
    pub description: String,
    pub np_handle: String,
    pub root_level: [u8; 20],
    pub icon: ResrcDescriptor,
    pub game: GameVersion,
    pub initially_locked: bool,
    pub is_sub_level: bool,
    pub background_guid: Option<u32>,
    pub shareable: bool,
    pub author_labels: Vec<u32>,
    pub leveltype: LevelType,
    pub min_players: Option<u8>,
    pub max_players: Option<u8>,
    pub is_adventure_planet: bool,
}

pub fn get_slot_info(id: i64, db_path: &Path) -> Result<SlotInfo> {
    if !db_path.exists() {
        return Err(anyhow!("Database file is missing, download it or check if the path in config.yml is correct"));
    }

    let db = sqlite::open(db_path)?;

    let query = "SELECT name, description, npHandle, rootLevel, icon, game, initiallyLocked,
        isSubLevel, background, shareable, authorLabels, leveltype, minPlayers, maxPlayers, isAdventurePlanet
        FROM slot WHERE id = ?";
    let mut statement = db.prepare(query)?;
    statement.bind((1, id))?;

    match statement.next()? {
        State::Done => return Err(anyhow!("Level not found")),
        State::Row => {},
    }

    let slot_info = SlotInfo {
        name: statement.read::<Option<String>, _>("name")?.unwrap_or_default(),
        description: statement.read::<Option<String>, _>("description")?.unwrap_or_default(),
        np_handle: statement.read::<String, _>("npHandle")?,
        root_level: statement.read::<Vec<u8>, _>("rootLevel")?.try_into().map_err(|_| anyhow!("invalid rootLevel in db"))?,
        icon: {
            let bytes = statement.read::<Vec<u8>, _>("icon")?;
            match bytes.len() {
                20 => ResrcDescriptor::Sha1(bytes.try_into().map_err(|_| anyhow!("invalid icon in db"))?),
                4 => {
                    let bytes = bytes.try_into().map_err(|_| anyhow!("invalid icon in db"))?;
                    ResrcDescriptor::Guid(u32::from_be_bytes(bytes))
                },
                0 => {
                    ResrcDescriptor::Guid(0)
                },
                _ => return Err(anyhow!("invalid icon in db")),
            }
        },
        game: {
            let int = statement.read::<i64, _>("game")?;
            match int {
                0 => GameVersion::Lbp1,
                1 => GameVersion::Lbp2,
                2 => GameVersion::Lbp3,
                _ => return Err(anyhow!("invalid game version in db")),
            }
        },
        initially_locked: statement.read::<i64, _>("initiallyLocked")? == 1,
        is_sub_level: statement.read::<i64, _>("isSubLevel")? == 1,
        background_guid: statement.read::<Option<i64>, _>("background")?.map(|i| i as u32),
        shareable: statement.read::<i64, _>("shareable")? == 1,
        author_labels: {
            let bytes = statement.read::<Option<Vec<u8>>, _>("authorLabels")?;
            let mut labels = Vec::with_capacity(5);

            if let Some(arr) = &bytes {
                let bits = arr.view_bits::<Lsb0>();
                for (i, key_id) in LABEL_LAMS_KEY_IDS.iter().enumerate() {
                    if bits[i] {
                        labels.push(*key_id);
                    }
                }
            }

            labels
        },
        leveltype: {
            let string = statement.read::<Option<String>, _>("leveltype")?;
            match string.as_deref() {
                None => LevelType::Cooperative,
                Some("versus") => LevelType::Versus,
                Some("cutscene") => LevelType::Cutscene,
                _ => return Err(anyhow!("invalid leveltype in db")),
            }
        },
        min_players: statement.read::<Option<i64>, _>("minPlayers")?.map(|i| i as u8),
        max_players: statement.read::<Option<i64>, _>("maxPlayers")?.map(|i| i as u8),
        is_adventure_planet: statement.read::<i64, _>("isAdventurePlanet")? == 1,
    };
    assert!(matches!(statement.next(), Ok(State::Done)));

    Ok(slot_info)
}