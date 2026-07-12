//! PNG decoding and deterministic GPU atlas assembly.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use image::ImageFormat;
use punctum_gpu::{GpuAtlas, GpuAtlasError, GpuResource, PixelRect, PixelSize, ResourceId, Rgba8};

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
    if images.is_empty() {
        return Err(AssetError::EmptyImageSet);
    }

    let width = images.iter().try_fold(0_u32, |width, (_, image)| {
        width
            .checked_add(image.size.width)
            .ok_or(AssetError::AtlasDimensionsOverflow)
    })?;
    let height = images
        .iter()
        .map(|(_, image)| image.size.height)
        .max()
        .ok_or(AssetError::EmptyImageSet)?;
    let size = PixelSize::new(width, height);
    let byte_len = u64::from(width) * u64::from(height) * 4;
    if byte_len > u64::from(u32::MAX) {
        return Err(AssetError::AtlasDimensionsOverflow);
    }
    let byte_len = usize::try_from(byte_len).map_err(|_| AssetError::AtlasDimensionsOverflow)?;
    let mut rgba8 = vec![0; byte_len];
    let mut resources = Vec::with_capacity(images.len());
    let atlas_row_bytes = width as usize * 4;
    let mut x = 0_u32;

    for &(id, image) in images {
        if image.size.is_empty() {
            return Err(AssetError::EmptyImage { id });
        }
        let image_row_bytes = image.size.width as usize * 4;
        for row in 0..image.size.height as usize {
            let source = row * image_row_bytes;
            let target = row * atlas_row_bytes + x as usize * 4;
            rgba8[target..target + image_row_bytes]
                .copy_from_slice(&image.rgba8[source..source + image_row_bytes]);
        }
        resources.push(GpuResource::new(
            id,
            PixelRect::new(x, 0, image.size.width, image.size.height),
        ));
        x += image.size.width;
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
    use punctum_gpu::{PixelRect, ResourceId, Rgba8};

    use super::{DecodedImage, build_atlas, decode_png};

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
    fn rejects_non_png_bytes() {
        assert!(decode_png(b"not a png").is_err());
    }
}
