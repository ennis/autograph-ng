//! Images
use std::cell::Cell;
use std::cmp::max;
use std::ptr;

use ash::vk;

use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwnedHandle;
use resource::Resource;
use sync::SyncGroup;
use device::Device;

mod description;
mod wrapper;
mod unbound;
mod dimension;
mod traits;

//--------------------------------------------------------------------------------------------------
// Image dimensions

/// **Borrowed from vulkano**
#[derive(Copy, Clone, Debug)]
pub enum Dimensions {
    Dim1d {
        width: u32,
        array_layers: u32,
    },
    Dim2d {
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
        array_layers: u32,
    },
}

impl Dimensions {
    #[inline]
    pub fn width(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { width } => width,
            Dimensions::Dim2d { width, .. } => width,
            Dimensions::Dim3d { width, .. } => width,
            Dimensions::Cubemap { size } => size,
        }
    }

    #[inline]
    pub fn height(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim2d { height, .. } => height,
            Dimensions::Dim3d { height, .. } => height,
            Dimensions::Cubemap { size } => size,
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
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim3d { depth, .. } => depth,
            Dimensions::Cubemap { .. } => 1,
        }
    }

    #[inline]
    pub fn width_height_depth(&self) -> [u32; 3] {
        [self.width(), self.height(), self.depth()]
    }

    #[inline]
    pub fn array_layers(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { array_layers, .. } => array_layers,
            Dimensions::Dim2d { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { array_layers, .. } => array_layers,
        }
    }

    /*#[inline]
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

    /// Builds the corresponding `ImageDimensionInfo`.
    #[inline]
    pub fn to_image_dimension_info(&self) -> ImageDimensionInfo {
        match *self {
            Dimensions::Dim1d { width } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width,
                    height: 1,
                    depth: 1,
                },
                image_type: vk::ImageType::Type1d,
                array_layers: 1,
            },
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width,
                    height: 1,
                    depth: 1,
                },
                image_type: vk::ImageType::Type1d,
                array_layers,
            },
            Dimensions::Dim2d { width, height } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                image_type: vk::ImageType::Type2d,
                array_layers: 1,
            },
            Dimensions::Dim2dArray {
                width,
                height,
                array_layers,
            } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                image_type: vk::ImageType::Type2d,
                array_layers,
            },
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width,
                    height,
                    depth,
                },
                image_type: vk::ImageType::Type3d,
                array_layers: 1,
            },
            Dimensions::Cubemap { size } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width: size,
                    height: size,
                    depth: 1,
                },
                image_type: vk::ImageType::Type2d,
                array_layers: 6,
            },
            Dimensions::CubemapArray { size, array_layers } => ImageDimensionInfo {
                extent: vk::Extent3D {
                    width: size,
                    height: size,
                    depth: 1,
                },
                image_type: vk::ImageType::Type2d,
                array_layers: 6 * array_layers,
            },
        }
    }*/

    /*/// Builds the corresponding `ViewType`.
    #[inline]
    pub fn to_view_type(&self) -> ViewType {
        match *self {
            Dimensions::Dim1d { .. } => ViewType::Dim1d,
            Dimensions::Dim1dArray { .. } => ViewType::Dim1dArray,
            Dimensions::Dim2d { .. } => ViewType::Dim2d,
            Dimensions::Dim2dArray { .. } => ViewType::Dim2dArray,
            Dimensions::Dim3d { .. } => ViewType::Dim3d,
            Dimensions::Cubemap { .. } => ViewType::Cubemap,
            Dimensions::CubemapArray { .. } => ViewType::CubemapArray,
        }
    }*/

    /*/// Returns the total number of texels for an image of these dimensions.
    #[inline]
    pub fn num_texels(&self) -> u32 {
        self.width() * self.height() * self.depth() * self.array_layers_with_cube()
    }*/
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

