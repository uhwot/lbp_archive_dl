use std::{fs::File, io::Write, path::Path};

use crate::db::{GameVersion, SlotInfo};

use byteorder::{LittleEndian, WriteBytesExt};
use anyhow::Result;

enum DataFormat<'a> {
    Array(u32, &'a [u8]),
    String(u32, &'a str),
    Integer(u32),
}

impl DataFormat<'_> {
    fn get_fmt_id(&self) -> [u8; 2] {
        match self {
            Self::Array(..) => [0x04, 0x00],
            Self::String(..) => [0x04, 0x02],
            Self::Integer(..) => [0x04, 0x04],
        }
    }

    fn get_data(&self) -> Vec<u8> {
        match self {
            Self::Array(max, a) => {
                assert!(a.len() as u32 <= *max);
                a.to_vec()
            },
            Self::String(max, s) => {
                if s.len() >= *max as usize {
                    format!("{}...\0", s.split_at(*max as usize - 4).0)
                } else {
                    format!("{s}\0")
                }.as_bytes().to_vec()
            },
            Self::Integer(i) => i.to_le_bytes().to_vec(),
        }
    }

    fn get_max_size(&self) -> u32 {
        match self {
            DataFormat::Array(max, _) => *max,
            DataFormat::String(max, _) => *max,
            DataFormat::Integer(_) => 4,
        }
    }
}

struct IndexEntry<'a> {
    key: &'a str,
    data: DataFormat<'a>,
}

const ENTRIES_LEN: usize = 10;

pub fn make_sfo(slot_info: &SlotInfo, display_name: &str, bkp_name: &str, dir: &Path, gamever: &GameVersion) -> Result<Vec<u8>> {
    let title = match slot_info.is_adventure_planet {
        false => format!("{} Dry Archive Level Backup", gamever.get_title()),
        true => format!("{} Dry Archive Adventure Backup", gamever.get_title()),
    };
    let subtitle = format!("{display_name} by {}", slot_info.np_handle);

    // these need to be in alphabetical order
    let entries: [IndexEntry; ENTRIES_LEN] = [
        IndexEntry {
            key: "ACCOUNT_ID",
            data: DataFormat::Array(16, b"0000000000000000")
        },
        IndexEntry {
            key: "ATTRIBUTE",
            data: DataFormat::Integer(0)
        },
        IndexEntry {
            key: "CATEGORY",
            data: DataFormat::String(4, "SD")
        },
        IndexEntry {
            key: "DETAIL",
            data: DataFormat::String(1024, &slot_info.description)
        },
        IndexEntry {
            key: "PARAMS",
            data: DataFormat::Array(1024, &[0u8; 1024])
        },
        IndexEntry {
            key: "PARAMS2",
            data: DataFormat::Array(1024, &[0u8; 12])
        },
        IndexEntry {
            key: "SAVEDATA_DIRECTORY",
            data: DataFormat::String(64, bkp_name)
        },
        IndexEntry {
            key: "SAVEDATA_LIST_PARAM",
            data: DataFormat::String(8, "")
        },
        IndexEntry {
            key: "SUB_TITLE",
            data: DataFormat::String(128, &subtitle)
        },
        IndexEntry {
            key: "TITLE",
            data: DataFormat::String(128, &title)
        },
    ];

    let mut key_offsets = [0; ENTRIES_LEN];
    let mut key_table = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        key_offsets[i] = key_table.len() as u16;
        key_table.write_all(entry.key.as_bytes())?;
        key_table.write_u8(0)?; // null terminator
    }

    let mut data_info = [(0, 0); ENTRIES_LEN];
    let mut data_table = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let data = entry.data.get_data();
        let size = data.len() as u32;
        let offset = data_table.len() as u32;

        data_table.write_all(&data)?;
        
        let pad = entry.data.get_max_size() - size;
        if pad != 0 {
            data_table.write_all(&b"\0".repeat(pad as usize))?;
        }

        data_info[i] = (size, offset);
    }

    let mut sfo = Vec::new();

    sfo.write_all(b"\0PSF")?;
    sfo.write_all(&[0x01, 0x01, 0x00, 0x00])?; // version 1.1
    sfo.write_u32::<LittleEndian>(0)?; // key table offset, to be written later
    sfo.write_u32::<LittleEndian>(0)?; // data table offset, to be written later
    sfo.write_u32::<LittleEndian>(ENTRIES_LEN as u32)?;

    // index table
    for (i, entry) in entries.iter().enumerate() {
        sfo.write_u16::<LittleEndian>(key_offsets[i])?;
        sfo.write_all(&entry.data.get_fmt_id())?;
        sfo.write_u32::<LittleEndian>(data_info[i].0)?; // data size
        sfo.write_u32::<LittleEndian>(entry.data.get_max_size())?;
        sfo.write_u32::<LittleEndian>(data_info[i].1)?; // data offset
    }

    let key_table_offset = sfo.len();
    sfo.write_all(&key_table)?;

    // align to 4 byte boundary
    let mut pad = sfo.len() % 4;
    if pad != 0 {
        pad = 4 - pad;
    }
    sfo.write_all(&b"\0".repeat(pad))?;

    let data_table_offset = sfo.len();
    sfo.write_all(&data_table)?;

    (&mut sfo[8..12]).write_u32::<LittleEndian>(key_table_offset as u32)?;
    (&mut sfo[12..16]).write_u32::<LittleEndian>(data_table_offset as u32)?;

    let mut file = File::create(dir.join("PARAM.SFO"))?;
    file.write_all(&sfo)?;

    Ok(sfo)
}
