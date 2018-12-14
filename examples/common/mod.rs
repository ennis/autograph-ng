use gfx2::app::*;
use gfx2::renderer;
use gfx2::renderer::*;
use image;
use image::GenericImageView;
use std::error;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum ImageLoadError {
    UnsupportedColorType(image::ColorType),
    Other(image::ImageError),
}

impl From<image::ImageError> for ImageLoadError {
    fn from(err: image::ImageError) -> Self {
        ImageLoadError::Other(err)
    }
}

impl fmt::Display for ImageLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ImageLoadError::UnsupportedColorType(color_type) => {
                write!(f, "unsupported color type: {:?}", color_type)
            }
            ImageLoadError::Other(err) => err.fmt(f),
        }
    }
}

impl error::Error for ImageLoadError {}

//
pub fn load_image_2d<'a, P: AsRef<Path>, R: RendererBackend>(
    arena: &'a Arena<R>,
    path: P,
) -> Result<Image<'a, R>, ImageLoadError> {
    let img = image::open(path)?;
    let (width, height) = img.dimensions();
    let format = match img.color() {
        image::ColorType::RGB(8) => Format::R8G8B8_SRGB,
        image::ColorType::RGBA(8) => Format::R8G8B8A8_SRGB,
        other => return Err(ImageLoadError::UnsupportedColorType(other)),
    };
    let bytes: &[u8] = match img {
        image::DynamicImage::ImageRgb8(ref rgb) => &*rgb,
        image::DynamicImage::ImageRgba8(ref rgba) => &*rgba,
        _ => panic!(""),
    };

    Ok(arena.create_immutable_image(
        format,
        (width, height).into(),
        MipmapsCount::One,
        1,
        ImageUsageFlags::SAMPLED,
        bytes,
    ))
}
