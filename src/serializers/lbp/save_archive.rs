use std::{collections::BTreeMap, fs::File, io::Write, path::Path};

use byteorder::{BigEndian, WriteBytesExt};
use hmac::Mac;

use crate::{resource_parse::ResrcRevision, serializers::HmacSha1, xxtea};

const TEA_KEY: [u32; 4] = [0x1B70CBD, 0x149607D6, 0x7F94DD5, 0x10DB8CA0];
const HASHINATE_KEY: [u8; 64] = [
    0x2A, 0xFD, 0xA3, 0xCA, 0x86, 0x02, 0x19, 0xB3,
    0xE6, 0x8A, 0xFF, 0xCC, 0x82, 0xC7, 0x6B, 0x8A,
    0xFE, 0x0A, 0xD8, 0x13, 0x5F, 0x60, 0x47, 0x5B,
    0xDF, 0x5D, 0x37, 0xBC, 0x57, 0x1C, 0xB5, 0xE7,
    0x96, 0x75, 0xD5, 0x28, 0xA2, 0xFA, 0x90, 0xED,
    0xDF, 0xA3, 0x45, 0xB4, 0x1F, 0xF9, 0x1F, 0x25,
    0xE7, 0x42, 0x45, 0x3B, 0x2B, 0xB5, 0x3E, 0x16,
    0xC9, 0x58, 0x19, 0x7B, 0xE7, 0x18, 0xC0, 0x80
];
const CHUNK_SIZE: usize = 0x240000;

struct ArchiveEntry {
    sha1: [u8; 20],
    offset: u32,
    size: u32,
}

pub fn make_savearchive(rev: &ResrcRevision, slt_hash: [u8; 20], hashes: BTreeMap<[u8; 20], Option<Vec<u8>>>, bkp_dir: &Path) {
    let mut arc = Vec::new();
    let mut entries = Vec::new();

    for (hash, resource) in hashes {
        let resource = match resource {
            Some(res) => res,
            None => continue,
        };
        let offset = arc.len();

        arc.write_all(&resource).unwrap();

        entries.push(ArchiveEntry {
            sha1: hash,
            offset: offset as u32,
            size: resource.len() as u32,
        });
    }

    // align to 4 byte boundary
    let mut pad = arc.len() % 4;
    if pad != 0 {
        pad = 4 - pad;
    }

    arc.write_all(&b"\0".repeat(pad)).unwrap();

    // save key
    arc.write_u32::<BigEndian>(rev.head).unwrap();
    arc.write_u16::<BigEndian>(rev.branch_id).unwrap();
    arc.write_u16::<BigEndian>(rev.branch_revision).unwrap();
    arc.write_u32::<BigEndian>(1).unwrap(); // localUserID
    arc.write_all(&[0u8; 0x4 * 0xa]).unwrap(); // deprecated1 int[10]
    arc.write_u32::<BigEndian>(0).unwrap(); // copied
    arc.write_u32::<BigEndian>(29).unwrap(); // root type value, SLOT_LIST
    arc.write_all(&[0u8; 0x4 * 0x3]).unwrap(); // deprecated2 int[3]
    arc.write_all(&slt_hash).unwrap();
    arc.write_all(&[0u8; 0x4 * 0xa]).unwrap(); // deprecated3 int[10]

    // fat entries
    for entry in &entries {
        arc.write_all(&entry.sha1).unwrap();
        arc.write_u32::<BigEndian>(entry.offset).unwrap();
        arc.write_u32::<BigEndian>(entry.size).unwrap();
    }

    // hashinate, to be written later
    let hashinate_offset = arc.len();
    arc.write_all(&[0u8; 0x14]).unwrap();
    arc.write_u32::<BigEndian>(entries.len() as u32).unwrap();
    arc.write_all(b"FAR4").unwrap();

    let mut mac = HmacSha1::new_from_slice(&HASHINATE_KEY).unwrap();
    mac.update(&arc);
    (&mut arc[hashinate_offset..hashinate_offset + 0x14]).write_all(&mac.finalize().into_bytes()).unwrap();

    let last_chunk_idx = arc.len() / CHUNK_SIZE;
    for (i, chunk) in arc.chunks_mut(CHUNK_SIZE).enumerate() {
        let mut xxtea_end = chunk.len();
        if i == last_chunk_idx {
            xxtea_end -= 4;
        }
        xxtea::encrypt(&TEA_KEY, &mut chunk[..xxtea_end]);

        let mut file = File::create(bkp_dir.join(i.to_string())).unwrap();
        file.write_all(chunk).unwrap();
    }
}