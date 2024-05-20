use std::{fs::File, io::Write, path::Path};

use aes::cipher::{block_padding::ZeroPadding, BlockEncryptMut, KeyIvInit};
use byteorder::{BigEndian, WriteBytesExt};
use hmac::{digest::{consts::U20, generic_array::GenericArray}, Mac};

use crate::serializers::HmacSha1;

// code based on
// https://gitlab.com/osyu/slotmachine/-/blob/master/slotmachine/pfd.py

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

const SYSCON_MANAGER_KEY: [u8; 16] = [0xd4, 0x13, 0xb8, 0x96, 0x63, 0xe1, 0xfe, 0x9f, 0x75, 0x14, 0x3d, 0x3b, 0xb4, 0x56, 0x52, 0x74];
const KEYGEN_KEY: [u8; 20] = [0x6b, 0x1a, 0xce, 0xa2, 0x46, 0xb7, 0x45, 0xfd, 0x8f, 0x93, 0x76, 0x3b, 0x92, 0x05, 0x94, 0xcd, 0x53, 0x48, 0x3b, 0x82];
const SAVEGAME_PARAM_SFO_KEY: [u8; 20] = [0x0c, 0x08, 0x00, 0x0e, 0x09, 0x05, 0x04, 0x04, 0x0d, 0x01, 0x0f, 0x00, 0x04, 0x06, 0x02, 0x02, 0x09, 0x06, 0x0d, 0x03];

fn hmac_digest(key: &[u8], data: &[u8]) -> GenericArray<u8, U20> {
    let mut hmac = HmacSha1::new_from_slice(key).unwrap();
    hmac.update(data);
    hmac.finalize().into_bytes()
}

pub fn make_pfd(version: u64, sfo: Vec<u8>, dir: &Path) {
    // these are normally random, but we can just null them out
    let pf_header_iv = [0u8; 16];
    let pf_key_orig = [0u8; 20];

    let mut pf_key = pf_key_orig;
    if version == 4 {
        pf_key.copy_from_slice(hmac_digest(&KEYGEN_KEY, &pf_key_orig).as_slice());
    }

    let pf_index_size = 1; // normally 57, we only need 1
    let pf_entry_size = 1; // normally 114, we only need 1

    let mut sfo_filename = [0u8; 65];
    sfo_filename[..9].copy_from_slice(b"PARAM.SFO");

    // the only protected file entry, for our PARAM.SFO
    let mut pf_entries = Vec::new();
    pf_entries.write_u64::<BigEndian>(pf_index_size).unwrap();
    pf_entries.write_all(&sfo_filename).unwrap();
    pf_entries.write_all(&b"\0".repeat(7)).unwrap(); // padding
    pf_entries.write_all(&b"\0".repeat(64)).unwrap(); // file encryption key
    pf_entries.write_all(hmac_digest(&SAVEGAME_PARAM_SFO_KEY, &sfo).as_slice()).unwrap();
    pf_entries.write_all(&b"\0".repeat(20)).unwrap(); // console id hash
    pf_entries.write_all(&b"\0".repeat(20)).unwrap(); // disc key hash
    pf_entries.write_all(&b"\0".repeat(20)).unwrap(); // account id hash
    pf_entries.write_all(&b"\0".repeat(40)).unwrap(); // reserved
    pf_entries.write_u64::<BigEndian>(sfo.len() as u64).unwrap();

    // protected file index
    let mut pf_index = Vec::new();
    pf_index.write_u64::<BigEndian>(pf_index_size).unwrap();
    pf_index.write_u64::<BigEndian>(pf_entry_size).unwrap(); // reserved entries
    pf_index.write_u64::<BigEndian>(pf_entry_size).unwrap(); // used entries
    pf_index.write_u64::<BigEndian>(0).unwrap(); // PARAM.SFO entry index

    let mut hmac = HmacSha1::new_from_slice(&pf_key).unwrap();
    // signature doesn't include next entry index or the padding after file name
    hmac.update(&sfo_filename);
    hmac.update(&pf_entries[80..]);
    // only one pf entry, so only one sig in the sig table
    let pf_entry_sig_table = hmac.finalize().into_bytes();

    // signature for pf index
    let pf_index_sig = hmac_digest(&pf_key, &pf_index);

    // signature for pf entry sig table
    let pf_entry_sig_table_sig = hmac_digest(&pf_key, &pf_entry_sig_table);

    let mut pf_header = Vec::new();
    pf_header.write_all(pf_entry_sig_table_sig.as_slice()).unwrap();
    pf_header.write_all(pf_index_sig.as_slice()).unwrap();
    pf_header.write_all(&pf_key_orig).unwrap();
    pf_header.write_all(b"\0\0\0\0").unwrap(); // padding

    Aes128CbcEnc::new(&SYSCON_MANAGER_KEY.into(), &pf_header_iv.into())
        .encrypt_padded_mut::<ZeroPadding>(&mut pf_header, 64)
        .unwrap();

    let mut file = File::create(dir.join("PARAM.PFD")).unwrap();

    file.write_all(b"\0\0\0\0PFDB").unwrap();
    file.write_u64::<BigEndian>(version).unwrap();

    file.write_all(&pf_header_iv).unwrap();
    file.write_all(&pf_header).unwrap();

    file.write_all(&pf_index).unwrap();
    file.write_all(&pf_entries).unwrap();
    file.write_all(&pf_entry_sig_table).unwrap();
}