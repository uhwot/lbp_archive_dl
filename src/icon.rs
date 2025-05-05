use std::{collections::BTreeMap, fs::File, io::{Cursor, Write}, path::Path};

use crate::{gtf_texture::make_dds_header, resource_parse::{ResrcData, ResrcMethod}};

use image::{imageops::FilterType, ImageReader, DynamicImage, ImageBuffer, ImageFormat, Rgba};
use anyhow::Result;

const MAX_WIDTH: u32 = 320;
const MAX_HEIGHT: u32 = 176;

// code epically stolen from here :D
// https://github.com/image-rs/image/issues/1701#issuecomment-1100276695
fn img_resize_with_padding(img: DynamicImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut width = img.width();
    let mut height = img.height();
    let aspect_ratio = (width as f32) / (height as f32);

    if width > MAX_WIDTH || height < MAX_HEIGHT {
        width = MAX_WIDTH;
        height = ((width as f32) / aspect_ratio) as u32;
    }

    if height > MAX_HEIGHT || width < MAX_WIDTH {
        height = MAX_HEIGHT;
        width = ((height as f32) * aspect_ratio) as u32;
    }

    let thumbnail = img.resize_exact(width, height, FilterType::Triangle);
    let mut img = ImageBuffer::from_fn(MAX_WIDTH, MAX_HEIGHT, |_x, _y| Rgba([0, 0, 0, 0]));
    image::imageops::overlay(
        &mut img,
        &thumbnail,
        ((MAX_WIDTH - width) / 2).into(),
        ((MAX_HEIGHT - height) / 2).into(),
    );
    img
}

pub fn make_icon(bkp_path: &Path, icon_hash: Option<[u8; 20]>, hashes: &mut BTreeMap<[u8; 20], Vec<u8>>) -> Result<()> {
    let mut icon_data = None;
    let mut icon_gcm_info = None;

    if let Some(hash) = icon_hash {
        if let Some(icon_resrc) = hashes.get(&hash) {
            let icon_resrc_id = ResrcData::new(icon_resrc, true)?;
            if let ResrcMethod::Texture { data, gcm_info } = icon_resrc_id.method {
                icon_data = Some(data);
                icon_gcm_info = gcm_info;
            }
        }
    }

    let mut icon_file = File::create(bkp_path.join("ICON0.PNG"))?;

    match icon_data {
        None => Ok(icon_file.write_all(include_bytes!("assets/placeholder_icon.png"))?),
        Some(mut data) => {
            if let Some(gcm_info) = icon_gcm_info {
                let mut dds = Vec::with_capacity(0x80 + data.len());
                make_dds_header(&mut dds, &gcm_info)?;
                dds.write_all(&data)?;

                data = dds;
            }

            let mut img = ImageReader::new(Cursor::new(data));
            img.set_format(ImageFormat::Dds);
            let img = img.decode()?;
            let img = img_resize_with_padding(img);
            img.write_to(&mut icon_file, ImageFormat::Png)?;
            
            Ok(())
        }
    }
}