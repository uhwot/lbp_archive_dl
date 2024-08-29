use std::{collections::BTreeMap, fs::File, io::{Cursor, Write}, path::Path};

use image::{imageops::FilterType, io::Reader as ImageReader, DynamicImage, ImageBuffer, ImageFormat, Rgba};

use crate::{gtf_texture::make_dds_header, resource_parse::{ResrcId, ResrcMethod}};

// code epically stolen from here :D
// https://github.com/image-rs/image/issues/1701#issuecomment-1100276695
fn img_resize_with_padding(img: DynamicImage) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let max_width = 320;
    let max_height = 176;
    let mut width = img.width();
    let mut height = img.height();
    let aspect_ratio = (width as f32) / (height as f32);

    if width > max_width || height < max_height {
        width = max_width;
        height = ((width as f32) / aspect_ratio) as u32;
    }

    if height > max_height || width < max_width {
        height = max_height;
        width = ((height as f32) * aspect_ratio) as u32;
    }

    let thumbnail = img.resize_exact(width, height, FilterType::Triangle);
    let mut img = ImageBuffer::from_fn(max_width, max_height, |_x, _y| image::Rgba([0, 0, 0, 0]));
    image::imageops::overlay(
        &mut img,
        &thumbnail,
        ((max_width - width) / 2).into(),
        ((max_height - height) / 2).into(),
    );
    img
}

pub fn make_icon(bkp_path: &Path, icon_hash: Option<[u8; 20]>, hashes: &mut BTreeMap<[u8; 20], Option<Vec<u8>>>) {
    let mut icon_data = None;
    let mut icon_gcm_info = None;

    if let Some(hash) = icon_hash {
        let icon_resrc = hashes.get(&hash).unwrap();
        if let Some(resrc) = icon_resrc {
            let icon_resrc_id = ResrcId::new(resrc, true);
            if let ResrcMethod::Texture { data, gcm_info } = icon_resrc_id.method {
                icon_data = Some(data);
                icon_gcm_info = gcm_info;
            }
        }
    }

    let mut icon_file = File::create(bkp_path.join("ICON0.PNG")).unwrap();

    match icon_data {
        None => icon_file.write_all(include_bytes!("assets/placeholder_icon.png")).unwrap(),
        Some(mut data) => {
            if let Some(gcm_info) = icon_gcm_info {
                let mut dds = Vec::with_capacity(0x80 + data.len());
                make_dds_header(&mut dds, &gcm_info);
                dds.write_all(&data).unwrap();

                data = dds;
            }

            let mut img = ImageReader::new(Cursor::new(data));
            img.set_format(ImageFormat::Dds);
            let img = img.decode().unwrap();
            let img = img_resize_with_padding(img);
            img.write_to(&mut icon_file, ImageFormat::Png).unwrap();
        }
    }
}