//! Images
use std::cell::Cell;
use std::cmp::max;
use std::ptr;

use ash::vk;

use alloc::{AllocatedMemory, AllocationCreateInfo, Allocator, HostAccess};
use context::{Context, FrameNumber, VkDevice1, FRAME_NONE};
use handle::OwnedHandle;
use resource::Resource;
use sync::SyncGroup;

mod description;
mod wrapper;

pub use self::description::ImageDescription;
pub use self::wrapper::Image;

//--------------------------------------------------------------------------------------------------
// Image dimensions
/// **Borrowed from vulkano**
struct ImageDimensionInfo {
    image_type: vk::ImageType,
    extent: vk::Extent3D,
    array_layers: u32,
}

/// **Borrowed from vulkano**
#[derive(Copy, Clone, Debug)]
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
    }

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

    /// Returns the total number of texels for an image of these dimensions.
    #[inline]
    pub fn num_texels(&self) -> u32 {
        self.width() * self.height() * self.depth() * self.array_layers_with_cube()
    }
}

/// **Borrowed from vulkano**
/// Specifies how many mipmaps must be allocated.
///
/// Note that at least one mipmap must be allocated, to store the main level of the image.
#[derive(Debug, Copy, Clone)]
pub enum MipmapsCount {
    /// Allocates the number of mipmaps required to store all the mipmaps of the image where each
    /// mipmap is half the dimensions of the previous level. Guaranteed to be always supported.
    ///
    /// Note that this is not necessarily the maximum number of mipmaps, as the Vulkan
    /// implementation may report that it supports a greater value.
    Log2,

    /// Allocate one mipmap (ie. just the main level). Always supported.
    One,

    /// Allocate the given number of mipmaps. May result in an error if the value is out of range
    /// of what the implementation supports.
    Specific(u32),
}

impl From<u32> for MipmapsCount {
    #[inline]
    fn from(num: u32) -> MipmapsCount {
        MipmapsCount::Specific(num)
    }
}

fn get_texture_mip_map_count(size: u32) -> u32 {
    1 + f32::floor(f32::log2(size as f32)) as u32
}

// image::dimensions
