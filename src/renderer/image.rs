//! Images
use std::cmp::max;
use std::fmt;
use std::ptr;

//--------------------------------------------------------------------------------------------------
// Image dimensions

/// **Borrowed from vulkano**
#[derive(Copy, Clone)]
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
    pub fn width_height(&self) -> [u32; 2] {
        [self.width(), self.height()]
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
    pub fn width_height_depth(&self) -> [u32; 3] {
        [self.width(), self.height(), self.depth()]
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

    /*#[inline]
    pub fn to_image_extents_and_type(&self) -> ImageExtentsAndType {
        match *self {
            Dimensions::Dim1d { width } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width,
                    height: 1,
                    depth: 1,
                },
                type_: vk::ImageType::Type1d,
                array_layers: 1,
            },
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width,
                    height: 1,
                    depth: 1,
                },
                type_: vk::ImageType::Type1d,
                array_layers,
            },
            Dimensions::Dim2d { width, height } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                type_: vk::ImageType::Type2d,
                array_layers: 1,
            },
            Dimensions::Dim2dArray {
                width,
                height,
                array_layers,
            } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                type_: vk::ImageType::Type2d,
                array_layers,
            },
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth,
                },
                type_: vk::ImageType::Type3d,
                array_layers: 1,
            },
            Dimensions::Cubemap { size } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width: size,
                    height: size,
                    depth: 1,
                },
                type_: vk::ImageType::Type2d,
                array_layers: 6,
            },
            Dimensions::CubemapArray { size, array_layers } => ImageExtentsAndType {
                extent: vk::Extent3D {
                    width: size,
                    height: size,
                    depth: 1,
                },
                type_: vk::ImageType::Type2d,
                array_layers: 6 * array_layers,
            },
        }
    }*/
}

impl fmt::Debug for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Dimensions::Dim1d { width } => {
                write!(f, "[1D {}x1]", width);
            }
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => {
                write!(f, "[1D Array {}x1(x{})]", width, array_layers);
            }
            Dimensions::Dim2d { width, height } => {
                write!(f, "[2D {}x{}]", width, height);
            }
            Dimensions::Dim2dArray {
                width,
                height,
                array_layers,
            } => {
                write!(f, "[2D Array {}x{}(x{})]", width, height, array_layers);
            }
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => {
                write!(f, "[3D {}x{}x{}]", width, height, depth);
            }
            Dimensions::Cubemap { size } => {
                write!(f, "[Cubemap {}x{}]", size, size);
            }
            Dimensions::CubemapArray { size, array_layers } => {
                write!(f, "[Cubemap Array {}x{}(x{})]", size, size, array_layers);
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

fn get_texture_mip_map_count(size: u32) -> u32 {
    1 + f32::floor(f32::log2(size as f32)) as u32
}

bitflags! {
    pub struct ImageUsageFlags: u32 {
        const COLOR_ATTACHMENT = 0b00000001;
        const DEPTH_ATTACHMENT = 0b00000010;
        const INPUT_ATTACHMENT = 0b00000100;
        const STORAGE          = 0b00001000;
        const SAMPLE           = 0b00010000;
    }
}
