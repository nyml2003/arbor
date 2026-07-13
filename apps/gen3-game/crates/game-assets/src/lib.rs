//! PNG decoding and deterministic GPU atlas assembly.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use image::ImageFormat;
use punctum_gpu::{GpuAtlas, GpuAtlasError, GpuResource, PixelRect, PixelSize, ResourceId, Rgba8};

const MAX_ATLAS_DIMENSION: u32 = 8_192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    size: PixelSize,
    rgba8: Vec<u8>,
}

impl DecodedImage {
    pub fn solid(color: Rgba8) -> Self {
        Self {
            size: PixelSize::new(1, 1),
            rgba8: color.to_array().to_vec(),
        }
    }

    pub const fn size(&self) -> PixelSize {
        self.size
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, AssetError> {
    let rgba = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(|error| AssetError::InvalidPng(error.to_string()))?
        .to_rgba8();
    Ok(DecodedImage {
        size: PixelSize::new(rgba.width(), rgba.height()),
        rgba8: rgba.into_raw(),
    })
}

pub fn build_atlas(images: &[(ResourceId, &DecodedImage)]) -> Result<GpuAtlas, AssetError> {
    build_atlas_with_limit(images, MAX_ATLAS_DIMENSION)
}

fn build_atlas_with_limit(
    images: &[(ResourceId, &DecodedImage)],
    maximum_width: u32,
) -> Result<GpuAtlas, AssetError> {
    if images.is_empty() {
        return Err(AssetError::EmptyImageSet);
    }
    if maximum_width == 0 {
        return Err(AssetError::AtlasDimensionsOverflow);
    }

    let mut placements = Vec::with_capacity(images.len());
    let mut x = 0_u32;
    let mut y = 0_u32;
    let mut row_height = 0_u32;
    let mut width = 0_u32;
    for &(id, image) in images {
        if image.size.is_empty() {
            return Err(AssetError::EmptyImage { id });
        }
        if image.size.width > maximum_width || image.size.height > MAX_ATLAS_DIMENSION {
            return Err(AssetError::AtlasDimensionsOverflow);
        }
        if x > 0 && x.saturating_add(image.size.width) > maximum_width {
            y = y
                .checked_add(row_height)
                .ok_or(AssetError::AtlasDimensionsOverflow)?;
            x = 0;
            row_height = 0;
        }
        placements.push((id, image, x, y));
        x = x
            .checked_add(image.size.width)
            .ok_or(AssetError::AtlasDimensionsOverflow)?;
        row_height = row_height.max(image.size.height);
        width = width.max(x);
    }
    let height = y
        .checked_add(row_height)
        .ok_or(AssetError::AtlasDimensionsOverflow)?;
    if height > MAX_ATLAS_DIMENSION {
        return Err(AssetError::AtlasDimensionsOverflow);
    }
    let size = PixelSize::new(width, height);
    let byte_len = u64::from(width) * u64::from(height) * 4;
    if byte_len > u64::from(u32::MAX) {
        return Err(AssetError::AtlasDimensionsOverflow);
    }
    let byte_len = usize::try_from(byte_len).map_err(|_| AssetError::AtlasDimensionsOverflow)?;
    let mut rgba8 = vec![0; byte_len];
    let mut resources = Vec::with_capacity(images.len());
    let atlas_row_bytes = width as usize * 4;

    for (id, image, x, y) in placements {
        let image_row_bytes = image.size.width as usize * 4;
        for row in 0..image.size.height as usize {
            let source = row * image_row_bytes;
            let target = (y as usize + row) * atlas_row_bytes + x as usize * 4;
            rgba8[target..target + image_row_bytes]
                .copy_from_slice(&image.rgba8[source..source + image_row_bytes]);
        }
        resources.push(GpuResource::new(
            id,
            PixelRect::new(x, y, image.size.width, image.size.height),
        ));
    }

    GpuAtlas::new(size, rgba8, &resources).map_err(AssetError::InvalidAtlas)
}

#[derive(Debug)]
pub enum AssetError {
    InvalidPng(String),
    EmptyImageSet,
    EmptyImage { id: ResourceId },
    AtlasDimensionsOverflow,
    InvalidAtlas(GpuAtlasError),
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPng(error) => write!(formatter, "invalid PNG: {error}"),
            Self::EmptyImageSet => formatter.write_str("an atlas requires at least one image"),
            Self::EmptyImage { id } => write!(formatter, "image {id:?} is empty"),
            Self::AtlasDimensionsOverflow => formatter.write_str("atlas dimensions overflow"),
            Self::InvalidAtlas(error) => write!(formatter, "invalid GPU atlas: {error}"),
        }
    }
}

impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidAtlas(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
    use punctum_gpu::{PixelRect, PixelSize, ResourceId, Rgba8};

    use super::{DecodedImage, build_atlas, build_atlas_with_limit, decode_png};

    #[test]
    fn decodes_png_pixels_as_rgba8() {
        let pixels = [255, 0, 0, 255, 0, 128, 255, 64];
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&pixels, 2, 1, ExtendedColorType::Rgba8)
            .unwrap();

        let image = decode_png(&png).unwrap();
        assert_eq!(image.size().width, 2);
        assert_eq!(image.size().height, 1);
        assert_eq!(image.rgba8(), pixels);
    }

    #[test]
    fn packs_images_in_stable_horizontal_order() {
        let white = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
        let red = DecodedImage::solid(Rgba8::new(255, 0, 0, 128));
        let atlas = build_atlas(&[(ResourceId(1), &white), (ResourceId(2), &red)]).unwrap();

        assert_eq!(
            atlas.resource(ResourceId(1)),
            Some(PixelRect::new(0, 0, 1, 1))
        );
        assert_eq!(
            atlas.resource(ResourceId(2)),
            Some(PixelRect::new(1, 0, 1, 1))
        );
        assert_eq!(atlas.rgba8(), &[255, 255, 255, 255, 255, 0, 0, 128]);
    }

    #[test]
    fn wraps_images_to_new_rows_before_the_texture_limit() {
        let white = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
        let red = DecodedImage::solid(Rgba8::new(255, 0, 0, 255));
        let blue = DecodedImage::solid(Rgba8::new(0, 0, 255, 255));
        let atlas = build_atlas_with_limit(
            &[
                (ResourceId(1), &white),
                (ResourceId(2), &red),
                (ResourceId(3), &blue),
            ],
            2,
        )
        .unwrap();

        assert_eq!(atlas.size(), PixelSize::new(2, 2));
        assert_eq!(
            atlas.resource(ResourceId(1)),
            Some(PixelRect::new(0, 0, 1, 1))
        );
        assert_eq!(
            atlas.resource(ResourceId(2)),
            Some(PixelRect::new(1, 0, 1, 1))
        );
        assert_eq!(
            atlas.resource(ResourceId(3)),
            Some(PixelRect::new(0, 1, 1, 1))
        );
    }

    #[test]
    fn rejects_non_png_bytes() {
        assert!(decode_png(b"not a png").is_err());
    }
}
