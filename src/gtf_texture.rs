use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};
use anyhow::{anyhow, Result};

const DDS_HEADER_FLAGS_TEXTURE: u32 = 0x00001007;
const DDS_HEADER_FLAGS_MIPMAP: u32 = 0x00020000;

const DDS_SURFACE_FLAGS_COMPLEX: u32 = 0x00000008;
const DDS_SURFACE_FLAGS_TEXTURE: u32 = 0x00001000;
const DDS_SURFACE_FLAGS_MIPMAP: u32 = 0x00400000;

const DDS_SURFACE_FLAGS_CUBEMAP: u32 = 0x00000200;
const DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEX: u32 = 0x00000400;
const DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEX: u32 = 0x00000800;
const DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEY: u32 = 0x00001000;
const DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEY: u32 = 0x00002000;
const DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEZ: u32 = 0x00004000;
const DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEZ: u32 = 0x00008000;

const DDS_FOURCC: u32 = 0x4;
const DDS_RGB: u32 = 0x40;
const DDS_RGBA: u32 = 0x41;
const DDS_LUMINANCE: u32 = 0x00020000;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CellGcmTexture {
    pub format: CellGcmEnumForGtf,
    pub mipmap: u8,
    pub dimension: u8,
    pub cubemap: u8,
    pub remap: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub location: u8,
    pub flags: u8,
    pub pitch: u32,
    pub offset: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum CellGcmEnumForGtf {
    B8,
    A1R5G5B5,
    A4R4G4B4,
    R5G6B5,
    A8R8G8B8,
    DXT1,
    DXT3,
    DXT5,
    G8B8,
    R5G5B5,
}

impl CellGcmEnumForGtf {
    pub fn from_u8(n: u8) -> Result<Self> {
        Ok(match n {
            0x81 => Self::B8,
            0x82 => Self::A1R5G5B5,
            0x83 => Self::A4R4G4B4,
            0x84 => Self::R5G6B5,
            0x85 => Self::A8R8G8B8,
            0x86 => Self::DXT1,
            0x87 => Self::DXT3,
            0x88 => Self::DXT5,
            0x8b => Self::G8B8,
            0x8f => Self::R5G5B5,
            _ => return Err(anyhow!("Invalid GTF texture pixel format")),
        })
    }
    fn dds_pixelformat(&self) -> Result<[u32; 8]> {
        Ok(match self {
            Self::B8 =>       [0x20, DDS_LUMINANCE, 0, 8, 0, 0, 0x000000ff, 0],
            Self::A1R5G5B5 => [0x20, DDS_RGBA, 0, 16, 0x00007c00, 0x000003e0, 0x0000001f, 0x00008000],
            Self::A4R4G4B4 => [0x20, DDS_RGBA, 0, 16, 0x00000f00, 0x000000f0, 0x0000000f, 0x0000f000],
            Self::R5G6B5 =>   [0x20, DDS_RGB, 0, 16, 0x0000f800, 0x000007e0, 0x0000001f, 0x00000000],
            Self::A8R8G8B8 => [0x20, DDS_RGBA, 0, 32, 0x00ff0000, 0x0000ff00, 0x000000ff, 0xff000000],
            Self::DXT1 =>     [0x20, DDS_FOURCC, 0x31545844, 0, 0, 0, 0, 0],
            Self::DXT3 =>     [0x20, DDS_FOURCC, 0x33545844, 0, 0, 0, 0, 0],
            Self::DXT5 =>     [0x20, DDS_FOURCC, 0x35545844, 0, 0, 0, 0, 0],
            _ => return Err(anyhow!("Unimplemented DDS pixel format type")),
        })
    }
}

// DDS header structure docs:
// https://docs.microsoft.com/en-us/windows/win32/direct3ddds/dds-header
pub fn make_dds_header(dds: &mut Vec<u8>, gcm: &CellGcmTexture) -> Result<()> {
    dds.write_all(b"DDS ")?;
    dds.write_u32::<LittleEndian>(0x7c)?; // dwSize
    
    let mut flags = DDS_HEADER_FLAGS_TEXTURE;
    if gcm.mipmap != 1 { flags |= DDS_HEADER_FLAGS_MIPMAP }
    dds.write_u32::<LittleEndian>(flags)?;

    dds.write_u32::<LittleEndian>(gcm.height.into())?;
    dds.write_u32::<LittleEndian>(gcm.width.into())?;
    dds.write_u32::<LittleEndian>(0)?; // dwPitchOrLinearSize
    dds.write_u32::<LittleEndian>(0)?; // dwDepth
    dds.write_u32::<LittleEndian>(gcm.mipmap.into())?;

    // dwReserved[11]
    dds.write_all(&[0u8; 11 * 4])?;

    for value in gcm.format.dds_pixelformat()? {
        dds.write_u32::<LittleEndian>(value)?;
    }

    let mut caps1 = DDS_SURFACE_FLAGS_TEXTURE;
    let mut caps2 = 0;

    if gcm.mipmap != 1 {
        caps1 |= DDS_SURFACE_FLAGS_MIPMAP;
        caps1 |= DDS_SURFACE_FLAGS_COMPLEX;
    }

    if gcm.cubemap == 1 {
        caps1 |= DDS_SURFACE_FLAGS_COMPLEX;

        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP;

        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEX;
        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEX;

        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEY;
        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEY;

        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_POSITIVEZ;
        caps2 |= DDS_SURFACE_FLAGS_CUBEMAP_NEGATIVEZ;
    }

    dds.write_u32::<LittleEndian>(caps1)?;
    dds.write_u32::<LittleEndian>(caps2)?;

    // dwReserved
    dds.write_all(&[0u8; 3 * 4])?;
    
    Ok(())
}