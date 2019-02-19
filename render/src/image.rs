use crate::Backend;
use bitflags::bitflags;
use std::fmt;

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
#[repr(transparent)]
pub struct Image<'a, B: Backend>(pub &'a B::Image);

impl<'a, B: Backend> Image<'a, B> {
    pub fn into_sampled(self, d: SamplerDescription) -> SampledImage<'a, B> {
        SampledImage(self.0, d)
    }
}

#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct SampledImage<'a, B: Backend>(pub &'a B::Image, pub SamplerDescription);

/// Dimensions of an image.
///
/// **Borrowed from vulkano**
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Dimensions {
    /// 1D image
    Dim1d { width: u32 },
    /// Array of 1D images
    Dim1dArray { width: u32, array_layers: u32 },
    /// 2D image
    Dim2d { width: u32, height: u32 },
    /// Array of 2D images
    Dim2dArray {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    /// 3D image
    Dim3d { width: u32, height: u32, depth: u32 },
    /// Cubemap image (6 2D images)
    Cubemap { size: u32 },
    /// Array of cubemaps
    CubemapArray { size: u32, array_layers: u32 },
}

impl Dimensions {
    /// Returns the width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { width } => width,
            Dimensions::Dim1dArray { width, .. } => width,
            Dimensions::Dim2d { width, .. } => width,
            Dimensions::Dim2dArray { width, .. } => width,
            Dimensions::Dim3d { width, .. } => width,
            Dimensions::Cubemap { size } => size,
            Dimensions::CubemapArray { size, .. } => size,
        }
    }

    /// Returns the height in pixels.
    ///
    /// Returns 1 for 1D images.
    #[inline]
    pub fn height(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { .. } => 1,
            Dimensions::Dim2d { height, .. } => height,
            Dimensions::Dim2dArray { height, .. } => height,
            Dimensions::Dim3d { height, .. } => height,
            Dimensions::Cubemap { size } => size,
            Dimensions::CubemapArray { size, .. } => size,
        }
    }

    /// Returns the (width,height) pair.
    ///
    /// Equivalent to `(self.width(), self.height())`
    #[inline]
    pub fn width_height(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Returns the depth (third dimension) of the image.
    ///
    /// Returns 1 for 1D, 2D or cubemap images.
    #[inline]
    pub fn depth(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { .. } => 1,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { .. } => 1,
            Dimensions::Dim3d { depth, .. } => depth,
            Dimensions::Cubemap { .. } => 1,
            Dimensions::CubemapArray { .. } => 1,
        }
    }

    /// Returns the (width,height,depth) triplet.
    ///
    /// Equivalent to `(self.width(), self.height(), self.depth())`
    #[inline]
    pub fn width_height_depth(&self) -> (u32, u32, u32) {
        (self.width(), self.height(), self.depth())
    }

    #[inline]
    pub fn array_layers(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { array_layers, .. } => array_layers,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { .. } => 1,
            Dimensions::CubemapArray { array_layers, .. } => array_layers,
        }
    }

    #[inline]
    pub fn array_layers_with_cube(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim1dArray { array_layers, .. } => array_layers,
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim2dArray { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { .. } => 6,
            Dimensions::CubemapArray { array_layers, .. } => array_layers * 6,
        }
    }
}

impl From<(u32, u32)> for Dimensions {
    fn from((width, height): (u32, u32)) -> Dimensions {
        Dimensions::Dim2d { width, height }
    }
}

impl fmt::Debug for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Dimensions::Dim1d { width } => {
                write!(f, "[1D {}x1]", width)?;
            }
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => {
                write!(f, "[1D Array {}x1(x{})]", width, array_layers)?;
            }
            Dimensions::Dim2d { width, height } => {
                write!(f, "[2D {}x{}]", width, height)?;
            }
            Dimensions::Dim2dArray {
                width,
                height,
                array_layers,
            } => {
                write!(f, "[2D Array {}x{}(x{})]", width, height, array_layers)?;
            }
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => {
                write!(f, "[3D {}x{}x{}]", width, height, depth)?;
            }
            Dimensions::Cubemap { size } => {
                write!(f, "[Cubemap {}x{}]", size, size)?;
            }
            Dimensions::CubemapArray { size, array_layers } => {
                write!(f, "[Cubemap Array {}x{}(x{})]", size, size, array_layers)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MipmapsCount {
    Log2,
    One,
    Specific(u32),
}

///
/// Get the maximum number of mip map levels for a 2D texture of size (width,height)
/// numLevels = 1 + floor(log2(max(w, h, d)))
///
/// # References
///
/// https://stackoverflow.com/questions/9572414/how-many-mipmaps-does-a-texture-have-in-opengl
pub fn get_texture_mip_map_count(size: u32) -> u32 {
    1 + f32::floor(f32::log2(size as f32)) as u32
}

bitflags! {
    pub struct ImageUsageFlags: u32 {
        const COLOR_ATTACHMENT = 0b0000_0001;
        const DEPTH_ATTACHMENT = 0b0000_0010;
        const INPUT_ATTACHMENT = 0b0000_0100;
        const STORAGE          = 0b0000_1000;
        const SAMPLED          = 0b0001_0000;
    }
}

impl Default for ImageUsageFlags {
    fn default() -> Self {
        ImageUsageFlags::COLOR_ATTACHMENT
            | ImageUsageFlags::DEPTH_ATTACHMENT
            | ImageUsageFlags::INPUT_ATTACHMENT
            | ImageUsageFlags::STORAGE
            | ImageUsageFlags::SAMPLED
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerAddressMode {
    Clamp,
    Mirror,
    Wrap,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerMipmapMode {
    Nearest,
    Linear,
}

// 2D sampler
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct SamplerDescription {
    pub addr_u: SamplerAddressMode,
    pub addr_v: SamplerAddressMode,
    pub addr_w: SamplerAddressMode,
    pub min_filter: Filter,
    pub mag_filter: Filter,
    pub mipmap_mode: SamplerMipmapMode,
}

impl SamplerDescription {
    pub const LINEAR_MIPMAP_LINEAR: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Linear,
        min_filter: Filter::Linear,
        mipmap_mode: SamplerMipmapMode::Linear,
    };

    pub const LINEAR_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Linear,
        min_filter: Filter::Linear,
        mipmap_mode: SamplerMipmapMode::Nearest,
    };

    pub const NEAREST_MIPMAP_LINEAR: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Linear,
    };

    pub const NEAREST_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Clamp,
        addr_v: SamplerAddressMode::Clamp,
        addr_w: SamplerAddressMode::Clamp,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Nearest,
    };

    pub const WRAP_NEAREST_MIPMAP_NEAREST: SamplerDescription = SamplerDescription {
        addr_u: SamplerAddressMode::Wrap,
        addr_v: SamplerAddressMode::Wrap,
        addr_w: SamplerAddressMode::Wrap,
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        mipmap_mode: SamplerMipmapMode::Nearest,
    };
}
