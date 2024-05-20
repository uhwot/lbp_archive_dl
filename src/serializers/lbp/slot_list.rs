use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

use crate::{db::{LevelType, SlotInfo}, resource_parse::ResrcRevision, ResrcDescriptor};

fn make_wstr(slt: &mut Vec<u8>, string: &str) {
    let wide_string: Vec<u16> = string.encode_utf16().collect();
    slt.write_u32::<BigEndian>(wide_string.len() as u32).unwrap();
    for i in wide_string {
        slt.write_u16::<BigEndian>(i).unwrap();
    }
}

fn make_str(slt: &mut Vec<u8>, string: &str) {
    let string = string.as_bytes();
    slt.write_u32::<BigEndian>(string.len() as u32).unwrap();
    slt.write_all(string).unwrap();
}

fn make_onlineid(slt: &mut Vec<u8>, rev: &ResrcRevision, np_handle: &str) {
    let length_prefixed = rev.get_version() < 0x234;
    if length_prefixed {
        slt.write_u32::<BigEndian>(16).unwrap();
    }

    let mut data = [0u8; 16];
    data[..np_handle.len()].copy_from_slice(np_handle.as_bytes());
    slt.write_all(&data).unwrap();

    slt.write_u8(0).unwrap(); // term

    if length_prefixed {
        slt.write_u32::<BigEndian>(3).unwrap();
    }
    slt.write_all(b"\0\0\0").unwrap(); // dummy
}

fn make_res_descriptor(slt: &mut Vec<u8>, rev: &ResrcRevision, deps: &mut Vec<(ResrcDescriptor, u32)>, desc: Option<ResrcDescriptor>, resrc_type: u32) {
    let mut hash = 1;
    let mut guid = 2;
    // why
    if rev.get_version() < 0x191 {
        hash = 2;
        guid = 1;
    }

    match desc {
        None => slt.write_u8(0).unwrap(),
        Some(ResrcDescriptor::Sha1(sha1)) => {
            slt.write_u8(hash).unwrap();
            slt.write_all(&sha1).unwrap();
        },
        Some(ResrcDescriptor::Guid(g)) => {
            slt.write_u8(guid).unwrap();
            slt.write_u32::<BigEndian>(g).unwrap();
        },
    }

    if let Some(desc) = desc {
        deps.push((desc, resrc_type));
    }
}

fn make_slot_struct(slt: &mut Vec<u8>, rev: &ResrcRevision, slot_info: &SlotInfo) -> Vec<(ResrcDescriptor, u32)> {
    let mut dependencies = Vec::new();
    let version = rev.get_version();
    let subversion = rev.get_subversion();

    // SlotID struct
    slt.write_u32::<BigEndian>(6).unwrap(); // slot type, FAKE
    slt.write_u32::<BigEndian>(0).unwrap(); // slot id

    let root_desc = match slot_info.is_adventure_planet {
        true => None,
        false => Some(ResrcDescriptor::Sha1(slot_info.root_level.as_slice().try_into().unwrap()))
    };
    make_res_descriptor(slt, rev, &mut dependencies, root_desc, 9);

    if subversion >= 0x145 {
        let adventure_desc = match slot_info.is_adventure_planet {
            true => Some(ResrcDescriptor::Sha1(slot_info.root_level.as_slice().try_into().unwrap())),
            false => None,
        };
        make_res_descriptor(slt, rev, &mut dependencies, adventure_desc, 31);
    }

    make_res_descriptor(slt, rev, &mut dependencies, Some(slot_info.icon), 1);

    // location, this shouldn't matter
    for _ in 0..4 {
        slt.write_f32::<BigEndian>(0.0).unwrap();
    }

    // authorID, NetworkOnlineID struct
    make_onlineid(slt, rev, &slot_info.np_handle);

    // authorName
    if version >= 0x13b {
        make_wstr(slt, &slot_info.np_handle);
    }

    make_str(slt, ""); // translationTag

    make_wstr(slt, &slot_info.name);
    make_wstr(slt, &slot_info.description);

    // primaryLinkLevel, SlotID, shouldn't matter?
    slt.write_u32::<BigEndian>(0).unwrap(); // DEVELOPER
    slt.write_u32::<BigEndian>(0).unwrap();

    // group, SlotID, shouldn't matter?
    if version >= 0x134 {
        slt.write_u32::<BigEndian>(0).unwrap(); // DEVELOPER
        slt.write_u32::<BigEndian>(0).unwrap();
    }

    slt.write_u8(slot_info.initially_locked as u8).unwrap();

    if version > 0x237 {
        slt.write_u8(slot_info.shareable as u8).unwrap();
        slt.write_u32::<BigEndian>(slot_info.background_guid.unwrap_or(0)).unwrap();
    }

    if version > 0x333 {
        make_res_descriptor(slt, rev, &mut dependencies, None, 38); // planetDecorations
    }

    if version < 0x188 {
        slt.write_u8(0).unwrap(); // unknown
    }

    if version > 0x1de {
        // developerLevelType
        slt.write_u32::<BigEndian>(match slot_info.leveltype {
            LevelType::Cooperative => 0, // MAIN_PATH
            LevelType::Versus => 6, // VERSUS
            LevelType::Cutscene => 7, // CUTSCENE
        }).unwrap();
    } else {
        slt.write_u8(false as u8).unwrap(); // SideMission
    }

    if version > 0x1ad && version < 0x1b9 {
        slt.write_u8(0).unwrap(); // unknown
    }

    if version > 0x1b8 && version < 0x36c {
        slt.write_u32::<BigEndian>(0).unwrap(); // gameProgressionState, NEW_GAME
    }

    if version <= 0x2c3 {
        return dependencies;
    }

    // labels
    if version >= 0x33c {
        slt.write_u32::<BigEndian>(slot_info.author_labels.len() as u32).unwrap();

        for (i, key_id) in slot_info.author_labels.iter().enumerate() {
            slt.write_u32::<BigEndian>(*key_id).unwrap();
            slt.write_u32::<BigEndian>(i as u32).unwrap();
        }
    }

    // collectabubblesRequired
    if version >= 0x2ea {
        slt.write_u32::<BigEndian>(3).unwrap(); // array count
        for _ in 0..3 {
            make_res_descriptor(slt, rev, &mut dependencies, None, 38); // null plan descriptor
            slt.write_u32::<BigEndian>(0).unwrap(); // count
        }
    }

    if version >= 0x2f4 {
        slt.write_u32::<BigEndian>(0).unwrap(); // collectabubblesContained
    }

    if version >= 0x352 {
        slt.write_u8(slot_info.is_sub_level as u8).unwrap();
    }

    if version < 0x3d0 {
        return dependencies;
    }

    slt.write_u8(slot_info.min_players.unwrap_or(1)).unwrap();
    slt.write_u8(slot_info.max_players.unwrap_or(4)).unwrap();

    if subversion >= 0x215 {
        slt.write_u8(false as u8).unwrap(); // enforceMinMaxPlayers
    }

    if version >= 0x3d0 {
        slt.write_u8(false as u8).unwrap(); // moveRecommended
    }

    if version >= 0x3e9 {
        slt.write_u8(false as u8).unwrap(); // crossCompatible
    }

    if version >= 0x3d1 {
        slt.write_u8(true as u8).unwrap(); // showOnPlanet
    }
    if version >= 0x3d2 {
        slt.write_u8(0).unwrap(); // livesOverride
    }

    if !rev.is_lbp3() {
        return dependencies;
    }

    if subversion >= 0x12 {
        // gameMode
        slt.write_u8(match slot_info.leveltype {
            LevelType::Cooperative => 0,
            LevelType::Versus => 1,
            LevelType::Cutscene => 2,
        }).unwrap(); 
    }

    if subversion >= 0xd2 {
        slt.write_u8(false as u8).unwrap(); // isGameKit
    }

    if subversion >= 0x11b {
        make_wstr(slt, ""); // entranceName
        // originalSlotID, SlotID struct
        slt.write_u32::<BigEndian>(0).unwrap(); // slot type, DEVELOPER
        slt.write_u32::<BigEndian>(0).unwrap(); // slot id
    }

    if subversion >= 0x153 {
        slt.write_u8(1).unwrap(); // customBadgeSize
    }

    if subversion >= 0x192 {
        make_str(slt, ""); // localPath
        if subversion >= 0x206 {
            make_str(slt, ""); // thumbPath
        }
    }

    dependencies
}

pub fn make_slotlist(rev: &ResrcRevision, slot_info: &SlotInfo) -> Vec<u8> {
    let mut slt = Vec::new();

    // resource header crap

    slt.write_all(b"SLTb").unwrap();
    slt.write_u32::<BigEndian>(rev.head).unwrap();
    
    if rev.head >= 0x109 {
        // dependency table offset, to be written later
        slt.write_u32::<BigEndian>(0).unwrap();

        if rev.head >= 0x189 {
            if rev.head >= 0x271 {
                slt.write_u16::<BigEndian>(rev.branch_id).unwrap();
                slt.write_u16::<BigEndian>(rev.branch_revision).unwrap();
            }

            if rev.head >= 0x297 || (rev.head == 0x272 && rev.branch_id == 0x4c44) && rev.branch_revision >= 0x2 {
                // compression flags
                slt.write_u8(0).unwrap();
            }

            // is compressed
            // yeah, i'm not compressing shit. it works anyways ¯\_(ツ)_/¯
            slt.write_u8(0).unwrap();
        }
    }

    // slotlist resource data

    // slot struct count, we just need one
    slt.write_u32::<BigEndian>(1).unwrap();

    let dependencies = make_slot_struct(&mut slt, rev, slot_info);

    if rev.get_version() >= 0x3b6 {
        slt.write_u8(true as u8).unwrap(); // fromProductionBuild
    }

    // dependency table

    if rev.head >= 0x109 {
        let dep_table_offset = slt.len();
        (&mut slt[8..12]).write_u32::<BigEndian>(dep_table_offset as u32).unwrap();

        slt.write_u32::<BigEndian>(dependencies.len() as u32).unwrap();
        for (dep, resrc_type) in dependencies {
            match dep {
                ResrcDescriptor::Sha1(sha1) => {
                    slt.write_u8(1).unwrap();
                    slt.write_all(&sha1).unwrap();
                },
                ResrcDescriptor::Guid(guid) => {
                    slt.write_u8(2).unwrap();
                    slt.write_u32::<BigEndian>(guid).unwrap();
                },
            }
            slt.write_u32::<BigEndian>(resrc_type).unwrap();
        }
    }

    slt
}