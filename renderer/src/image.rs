//! Images
use bitflags::bitflags;
use std::fmt;

//--------------------------------------------------------------------------------------------------
// Image dimensions

/// **Borrowed from vulkano**
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Dimensions {
    Dim1d {
        width: u32,
    },
    Dim1dArray {
        width: u32,
        array_layers: u32,
    },
    Dim2d {
        width: u32,
        height: u32,
    },
    Dim2dArray {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    Dim3d {
        width: u32,
        height: u32,
        depth: u32,
    },
    Cubemap {
        size: u32,
    },
    CubemapArray {
        size: u32,
        array_layers: u32,
    },
}

impl Dimensions {
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

    #[inline]
    pub fn width_height(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

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
