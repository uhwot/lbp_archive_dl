use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::{db::GameVersion, gtf_texture::{CellGcmEnumForGtf, CellGcmTexture}};

use byteorder::{BigEndian, ReadBytesExt};
use miniz_oxide::inflate::core::{decompress, DecompressorOxide};
use miniz_oxide::inflate::core::inflate_flags::{TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF};
use anyhow::{anyhow, Result};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ResrcData {
    pub resrc_type: [u8; 3],
    pub method: ResrcMethod,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ResrcRevision {
    pub head: u32,
    pub branch_id: u16,
    pub branch_revision: u16,
}

impl ResrcRevision {
    pub fn get_version(&self) -> u16 {
        (self.head & 0xFFFF) as u16
    }
    pub fn get_subversion(&self) -> u16 {
        ((self.head >> 16) & 0xFFFF) as u16
    }
    pub fn is_lbp1(&self) -> bool {
        self.head <= 0x272
    }
    pub fn is_lbp3(&self) -> bool {
        self.head >> 0x10 != 0
    }
    pub fn get_gameversion(&self) -> GameVersion {
        if self.is_lbp1() {
            GameVersion::Lbp1
        } else if self.is_lbp3() {
            GameVersion::Lbp3
        } else {
            GameVersion::Lbp2
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ResrcMethod {
    Null,
    Binary {
        is_encrypted: bool,
        revision: ResrcRevision,
        dependencies: Vec<ResrcDependency>,
    },
    Texture {
        data: Vec<u8>,
        gcm_info: Option<CellGcmTexture>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ResrcDependency {
    pub desc: ResrcDescriptor,
    resrc_type: u32,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ResrcDescriptor {
    Sha1([u8; 20]),
    Guid(u32),
}

impl ResrcDependency {
    pub fn parse_table(res: &mut Cursor<&[u8]>) -> Result<Vec<Self>> {
        let table_offset = res.read_u32::<BigEndian>()?;
        let orig_offset = res.position();

        res.seek(SeekFrom::Start(table_offset as u64))?;

        let mut dependencies = vec![];
        for _ in 0..res.read_u32::<BigEndian>()? {
            let dep_type = match res.read_u8()? {
                0 => { // lbp3 dynamic thermometer levels use this??? why??????
                    res.seek(SeekFrom::Current(4))?; // resrc_type
                    continue;
                }, 
                1 => {
                    let mut sha1 = [0u8; 20];
                    res.read_exact(&mut sha1)?;
                    ResrcDescriptor::Sha1(sha1)
                },
                2 => ResrcDescriptor::Guid(res.read_u32::<BigEndian>()?),
                _ => return Err(anyhow!("invalid type in dependency table, what the fuck???")),
            };

            let resrc_type = res.read_u32::<BigEndian>()?;

            dependencies.push(Self {
                desc: dep_type,
                resrc_type,
            })
        }

        res.seek(SeekFrom::Start(orig_offset))?;

        Ok(dependencies)
    }
}

impl ResrcData {
    pub fn new(res: &[u8], parse_texture: bool) -> Result<Self> {
        let mut res = Cursor::new(res);

        let mut resrc_type = [0u8; 3];
        res.read_exact(&mut resrc_type)?;

        let method = res.read_u8()?;

        let method = match method {
            b'b' | b'e' => {
                let mut rev = ResrcRevision {
                    head: res.read_u32::<BigEndian>()?,
                    branch_id: 0,
                    branch_revision: 0,
                };
                let dependencies = match rev.head >= 0x109 {
                    true => ResrcDependency::parse_table(&mut res)?,
                    false => vec![],
                };

                if resrc_type != *b"SMH" && rev.head >= 0x271 {
                    rev.branch_id = res.read_u16::<BigEndian>()?;
                    rev.branch_revision = res.read_u16::<BigEndian>()?;
                }

                ResrcMethod::Binary {
                    is_encrypted: method == b'e',
                    revision: rev,
                    dependencies,
                }
            },
            b' ' => {
                // decompressing texture data is only useful here for parsing the level icon,
                // otherwise we just skip decompression entirely
                if !parse_texture {
                    ResrcMethod::Null
                } else {
                    assert!([*b"TEX", *b"GTF"].contains(&resrc_type));

                    let mut gcm = None;

                    if resrc_type != *b"TEX" {
                        gcm = Some(CellGcmTexture {
                            format: CellGcmEnumForGtf::from_u8(res.read_u8()?)?,
                            mipmap: res.read_u8()?,
                            dimension: res.read_u8()?,
                            cubemap: res.read_u8()?,
                            remap: res.read_u32::<BigEndian>()?,
                            width: res.read_u16::<BigEndian>()?,
                            height: res.read_u16::<BigEndian>()?,
                            depth: res.read_u16::<BigEndian>()?,
                            location: res.read_u8()?,

                            flags: res.read_u8()?,

                            pitch: res.read_u32::<BigEndian>()?,
                            offset: res.read_u32::<BigEndian>()?,
                        });
                    }

                    res.seek(SeekFrom::Current(2))?; // unused i16, always 0x0001
                    let num_chunks = res.read_u16::<BigEndian>()?;

                    let mut chunk_infos = Vec::with_capacity(num_chunks as usize);
                    let mut total_decompressed_size = 0;

                    #[derive(Debug)]
                    struct ChunkInfo {
                        compressed_size: u16,
                        decompressed_size: u16,
                    }

                    for _ in 0..num_chunks {
                        let info = ChunkInfo {
                            compressed_size: res.read_u16::<BigEndian>()?,
                            decompressed_size: res.read_u16::<BigEndian>()?,
                        };
                        total_decompressed_size += info.decompressed_size as usize;
                        chunk_infos.push(info);
                    }

                    let mut final_data = vec![0u8; total_decompressed_size];

                    let mut decompressor = DecompressorOxide::new();

                    let mut final_pos = 0;
                    for info in chunk_infos {
                        let mut deflated_data = vec![0u8; info.compressed_size as usize];
                        res.read_exact(&mut deflated_data[..info.compressed_size as usize])?;

                        if info.compressed_size == info.decompressed_size {
                            (&mut final_data[final_pos..]).write_all(&deflated_data)?;
                        } else {
                            let flags = TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
                            decompress(&mut decompressor, &deflated_data, &mut final_data, final_pos, flags);
                            decompressor.init();
                        }

                        final_pos += info.decompressed_size as usize;
                    }

                    ResrcMethod::Texture { data: final_data, gcm_info: gcm }
                }
            },
            _ => { ResrcMethod::Null },
        };

        Ok(Self {
            resrc_type,
            method,
        })
    }
}