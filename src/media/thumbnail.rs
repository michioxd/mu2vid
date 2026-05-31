use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::{DynamicImage, Rgb, RgbImage};
use std::path::Path;

const THUMBNAIL_WIDTH: u32 = 1280;
const THUMBNAIL_HEIGHT: u32 = 720;

pub fn generate_thumbnail(artwork_path: &Path, output_path: &Path) -> Result<()> {
    let artwork = image::open(artwork_path)
        .with_context(|| format!("Cannot open artwork: {}", artwork_path.display()))?;
    let background = dark_background_color(&artwork);
    let resized = artwork
        .resize(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, FilterType::Lanczos3)
        .to_rgb8();
    let mut thumbnail = RgbImage::from_pixel(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, Rgb(background));
    let x = (THUMBNAIL_WIDTH - resized.width()) / 2;
    let y = (THUMBNAIL_HEIGHT - resized.height()) / 2;

    for row in 0..resized.height() {
        for col in 0..resized.width() {
            thumbnail.put_pixel(x + col, y + row, *resized.get_pixel(col, row));
        }
    }

    thumbnail
        .save(output_path)
        .with_context(|| format!("Cannot write thumbnail: {}", output_path.display()))
}

fn dark_background_color(image: &DynamicImage) -> [u8; 3] {
    let sample = image.thumbnail(64, 64).to_rgb8();
    let mut red = 0u64;
    let mut green = 0u64;
    let mut blue = 0u64;
    let mut count = 0u64;

    for pixel in sample.pixels() {
        red += pixel[0] as u64;
        green += pixel[1] as u64;
        blue += pixel[2] as u64;
        count += 1;
    }

    if count == 0 {
        return [30, 30, 30];
    }

    let darken = |value: u64| ((value / count) as f32 * 0.4).round().clamp(0.0, 255.0) as u8;
    [darken(red), darken(green), darken(blue)]
}
