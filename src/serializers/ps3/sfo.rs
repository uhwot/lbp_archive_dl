use std::{fs::File, io::Write, path::Path};

use byteorder::{LittleEndian, WriteBytesExt};

use crate::db::{GameVersion, SlotInfo};

enum DataFormat<'a> {
    Array(u32, &'a [u8]),
    String(u32, &'a str),
    Integer(u32),
}

impl<'a> DataFormat<'a> {
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

pub fn make_sfo(slot_info: &SlotInfo, bkp_name: &str, dir: &Path, gamever: &GameVersion) -> Vec<u8> {
    let title = match slot_info.is_adventure_planet {
        false => format!("{} Dry Archive Level Backup", gamever.get_title()),
        true => format!("{} Dry Archive Adventure Backup", gamever.get_title()),
    };
    let subtitle = format!("{} by {}", slot_info.name, slot_info.np_handle);

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
            data: DataFormat::Array(1024, b"\0")
        },
        IndexEntry {
            key: "PARAMS2",
            data: DataFormat::Array(1024, b"\0")
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
        key_table.write_all(entry.key.as_bytes()).unwrap();
        key_table.write_u8(0).unwrap(); // null terminator
    }

    let mut data_info = [(0, 0); ENTRIES_LEN];
    let mut data_table = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let data = entry.data.get_data();
        let size = data.len() as u32;
        let offset = data_table.len() as u32;

        data_table.write_all(&data).unwrap();
        
        let pad = entry.data.get_max_size() - size;
        if pad != 0 {
            data_table.write_all(&b"\0".repeat(pad as usize)).unwrap();
        }

        data_info[i] = (size, offset);
    }

    let mut sfo = Vec::new();

    sfo.write_all(b"\0PSF").unwrap();
    sfo.write_all(&[0x01, 0x01, 0x00, 0x00]).unwrap(); // version 1.1
    sfo.write_u32::<LittleEndian>(0).unwrap(); // key table offset, to be written later
    sfo.write_u32::<LittleEndian>(0).unwrap(); // data table offset, to be written later
    sfo.write_u32::<LittleEndian>(ENTRIES_LEN as u32).unwrap();

    // index table
    for (i, entry) in entries.iter().enumerate() {
        sfo.write_u16::<LittleEndian>(key_offsets[i]).unwrap();
        sfo.write_all(&entry.data.get_fmt_id()).unwrap();
        sfo.write_u32::<LittleEndian>(data_info[i].0).unwrap(); // data size
        sfo.write_u32::<LittleEndian>(entry.data.get_max_size()).unwrap();
        sfo.write_u32::<LittleEndian>(data_info[i].1).unwrap(); // data offset
    }

    let key_table_offset = sfo.len();
    sfo.write_all(&key_table).unwrap();

    // align to 4 byte boundary
    let mut pad = sfo.len() % 4;
    if pad != 0 {
        pad = 4 - pad;
    }
    sfo.write_all(&b"\0".repeat(pad)).unwrap();

    let data_table_offset = sfo.len();
    sfo.write_all(&data_table).unwrap();

    (&mut sfo[8..12]).write_u32::<LittleEndian>(key_table_offset as u32).unwrap();
    (&mut sfo[12..16]).write_u32::<LittleEndian>(data_table_offset as u32).unwrap();

    let mut file = File::create(dir.join("PARAM.SFO")).unwrap();
    file.write_all(&sfo).unwrap();

    sfo
}
