use std::{collections::HashMap, fs};

use image::RgbaImage;
use serde::Deserialize;

use crate::{config_parse::Config, profiling::Dropwatch};

const SUPPORTED_EXTENSIONS: [&'static str; 3] = ["png", "webp", "jpg"];

/// Gets the Texture of the atlas and the atlas entry metadata
pub fn read_atlas_data(
    cfg: &Config,
) -> Result<(RgbaImage, Vec<AtlasEntry>), Box<dyn std::error::Error>> {
    let atlas_texture = read_atlas_texture(&cfg)?;
    let atlas_json = read_atlas_json(&cfg)?;

    return Ok((atlas_texture, atlas_json));
}

fn read_atlas_texture(cfg: &Config) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let _d = Dropwatch::new("Atlas texture read");
    let provided_atlas_texture_fp = cfg.atlas_texture_fp.clone();

    // Make sure file has a filetype ext
    if provided_atlas_texture_fp.extension().is_none() {
        return Err(format!(
            "Provided atlas texture path: '{}' has no file extension!",
            provided_atlas_texture_fp.display()
        )
        .into());
    }

    // Fail on unsupported file type, collect all supported ones and display them to user
    let ext = provided_atlas_texture_fp
        .extension()
        .unwrap()
        .to_str()
        .unwrap(); // .unwrap() is ok here because we guarantee something exists above
    if !SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
        let display_text = SUPPORTED_EXTENSIONS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Provided atlas texture path has unsupported file type!: '{}' Supported types are: [{}]",
            ext,
            display_text
        )
        .into());
    }

    // Check if the provided filepath even links to anything
    if !std::fs::exists(&provided_atlas_texture_fp)? {
        return Err(format!(
            "Could not find atlas texture file at provided path: '{}'",
            provided_atlas_texture_fp.display()
        )
        .into());
    }

    // Attempt to load image file
    let atlas_texture = image::open(provided_atlas_texture_fp)?;

    return Ok(atlas_texture.to_rgba8());
}

#[repr(C)]
#[derive(Deserialize, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct AtlasEntry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rotated: u32,
}
impl std::fmt::Display for AtlasEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "x: {}\ny: {}\nw: {}\nh: {}\nr: {}",
            self.x, self.y, self.width, self.height, self.rotated
        )
    }
}

fn read_atlas_json(cfg: &Config) -> Result<Vec<AtlasEntry>, Box<dyn std::error::Error>> {
    let _d = Dropwatch::new("Atlas json read");
    let provided_atlas_json_fp = cfg.atlas_json_fp.clone();

    // Make sure file has a filetype ext
    if provided_atlas_json_fp.extension().is_none() {
        return Err(format!(
            "Provided atlas json path: '{}' has no file extension!",
            provided_atlas_json_fp.display()
        )
        .into());
    }

    // Make sure it is actually a .json file
    let ext = provided_atlas_json_fp
        .extension()
        .unwrap()
        .to_str()
        .unwrap(); // .unwrap() is ok here because we guarantee something exists above
    if ext.to_lowercase() != "json" {
        return Err(format!("Provided atlas json file is not a json file!: '{}'", ext).into());
    }

    // Check if the provided filepath even links to anything
    if !std::fs::exists(&provided_atlas_json_fp)? {
        return Err(format!(
            "Could not find atlas texture file at provided path: '{}'",
            provided_atlas_json_fp.display()
        )
        .into());
    }

    // Due to the json layout, we have to read it into a hashmap, and then move the values from the hashmap into an array
    let buffer = fs::read_to_string(provided_atlas_json_fp)?;
    let entries: Vec<AtlasEntry> = serde_json::from_str(&buffer)?;

    return Ok(entries);
}

/// Attempts to load the user specified image, calculates the downscale factor required to meet the pixel target and applies said downscaling before returning the image
/// cfg.extra_data.downscale_factor is set here
pub fn read_target_texture(cfg: &mut Config) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let _d = Dropwatch::new("Target texture read");
    let provided_target_texture_fp = cfg.target_texture.clone();

    // Make sure file has a filetype ext
    if provided_target_texture_fp.extension().is_none() {
        return Err(format!(
            "Provided target texture path: '{}' has no file extension!",
            provided_target_texture_fp.display()
        )
        .into());
    }

    // Fail on unsupported file type, collect all supported ones and display them to user
    let ext = provided_target_texture_fp
        .extension()
        .unwrap()
        .to_str()
        .unwrap(); // .unwrap() is ok here because we guarantee something exists above
    if !SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
        let display_text = SUPPORTED_EXTENSIONS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "Provided target texture path has unsupported file type!: '{}' Supported types are: [{}]",
            ext,
            display_text
        )
        .into());
    }

    // Check if the provided filepath even links to anything
    if !std::fs::exists(&provided_target_texture_fp)? {
        return Err(format!(
            "Could not find target texture file at provided path: '{}'",
            provided_target_texture_fp.display()
        )
        .into());
    }

    // Attempt to load image file
    let mut target_texture = image::open(provided_target_texture_fp)?;

    // See documentation of cfg.pixel_target for details
    let base_pixel_count = target_texture.width() * target_texture.height();
    if base_pixel_count < cfg.pixel_target {
        println!(
            "Automatically clamped pixel_target from {} down to {} as it was larger than the total number of pixels in the target image",
            cfg.pixel_target, base_pixel_count
        );
        cfg.pixel_target = base_pixel_count;
    }
    cfg.extra_data.downscale_factor = (base_pixel_count as f32 / cfg.pixel_target as f32).sqrt();

    // Downscale target
    target_texture = target_texture.resize(
        (target_texture.width() as f32 / cfg.extra_data.downscale_factor) as u32,
        (target_texture.height() as f32 / cfg.extra_data.downscale_factor) as u32,
        image::imageops::FilterType::Lanczos3,
    );

    return Ok(target_texture.to_rgba8());
}
