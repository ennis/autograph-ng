use crate::{
    descriptor::{
        Descriptor, ResourceBindingType, ResourceInterface, ResourceShape, SubresourceRange,
    },
    format::Format,
    typedesc::*,
    AliasScope, Arena, Backend,
};
use bitflags::bitflags;
use std::{
    cmp::max,
    fmt,
    ops::{Bound, RangeBounds},
};

#[derive(Copy, Clone, Debug)]
pub struct ImageCreateInfo<'a> {
    pub scope: AliasScope,
    pub format: Format,
    pub dimensions: Dimensions,
    pub mipmaps: MipmapsOption,
    pub samples: u32,
    pub usage: ImageUsageFlags,
    pub data: Option<&'a [u8]>,
}

/// An image.
#[derive(derivative::Derivative)]
#[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
pub struct GenericImage<'a, B: Backend> {
    pub(crate) inner: &'a B::Image,
    pub(crate) flags: ImageUsageFlags,
    pub(crate) shape: ResourceShape,
    pub(crate) format: Format,
}

impl<'a, B: Backend> GenericImage<'a, B> {
    pub fn inner(&self) -> &'a B::Image {
        self.inner
    }
    pub fn flags(&self) -> ImageUsageFlags {
        self.flags
    }
    pub fn is_render_target(&self) -> bool {
        self.flags.intersects(ImageUsageFlags::COLOR_ATTACHMENT)
    }
    pub fn is_texture(&self) -> bool {
        self.flags.intersects(ImageUsageFlags::SAMPLED)
    }
    pub fn is_read_write(&self) -> bool {
        self.flags.intersects(ImageUsageFlags::STORAGE)
    }
    pub fn shape(&self) -> ResourceShape {
        self.shape
    }
}

/// Dimensions of an image.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Dimensions {
    /// 1D image
    Dim1d { width: u32, array_layers: u32 },
    /// 2D image
    Dim2d {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    /// 3D image
    Dim3d { width: u32, height: u32, depth: u32 },
    /// Cubemap image (6 2D images)
    Cubemap { size: u32, array_layers: u32 },
}

impl Dimensions {
    /// Returns the width in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { width, .. } => width,
            Dimensions::Dim2d { width, .. } => width,
            Dimensions::Dim3d { width, .. } => width,
            Dimensions::Cubemap { size, .. } => size,
        }
    }

    /// Returns the height in pixels.
    ///
    /// Returns 1 for 1D images.
    #[inline]
    pub fn height(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { .. } => 1,
            Dimensions::Dim2d { height, .. } => height,
            Dimensions::Dim3d { height, .. } => height,
            Dimensions::Cubemap { size, .. } => size,
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
            Dimensions::Dim2d { .. } => 1,
            Dimensions::Dim3d { depth, .. } => depth,
            Dimensions::Cubemap { .. } => 1,
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
            Dimensions::Dim1d { array_layers, .. } => array_layers,
            Dimensions::Dim2d { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { array_layers, .. } => array_layers,
        }
    }

    #[inline]
    pub fn array_layers_with_cube(&self) -> u32 {
        match *self {
            Dimensions::Dim1d { array_layers, .. } => array_layers,
            Dimensions::Dim2d { array_layers, .. } => array_layers,
            Dimensions::Dim3d { .. } => 1,
            Dimensions::Cubemap { array_layers, .. } => array_layers * 6,
        }
    }
}

impl From<(u32, u32)> for Dimensions {
    fn from((width, height): (u32, u32)) -> Dimensions {
        Dimensions::Dim2d {
            width,
            height,
            array_layers: 1,
        }
    }
}

impl fmt::Debug for Dimensions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Dimensions::Dim1d {
                width,
                array_layers,
            } => {
                if *array_layers == 1 {
                    write!(f, "[1D {}]", width)
                } else {
                    write!(f, "[1D Array {}(x{})]", width, array_layers)
                }
            }
            Dimensions::Dim2d {
                width,
                height,
                array_layers,
            } => {
                if *array_layers == 1 {
                    write!(f, "[2D {}x{}]", width, height)
                } else {
                    write!(f, "[2D Array {}x{}(x{})]", width, height, array_layers)
                }
            }
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => write!(f, "[3D {}x{}x{}]", width, height, depth),
            Dimensions::Cubemap { size, array_layers } => {
                if *array_layers == 1 {
                    write!(f, "[Cubemap {}x{}]", size, size)
                } else {
                    write!(f, "[Cubemap Array {}x{}(x{})]", size, size, array_layers)
                }
            }
        }
    }
}

/*
#[derive(Debug, Copy, Clone)]
pub enum MipmapsCount {
    Log2,
    One,
    Specific(u32),
}*/

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

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MipmapsOption {
    NoMipmap,
    Allocate,
    AllocateCount(u32),
    Generate,
    GenerateCount(u32),
}

impl MipmapsOption {
    pub fn count(&self, width: u32, height: u32, depth: u32) -> u32 {
        match self {
            MipmapsOption::Allocate | MipmapsOption::Generate => {
                // TODO mipcount for 3D textures?
                get_texture_mip_map_count(max(width, height))
            }
            MipmapsOption::GenerateCount(n) | MipmapsOption::AllocateCount(n) => *n,
            MipmapsOption::NoMipmap => 1,
        }
    }
}

macro_rules! impl_image_builder {
    (@T size D1) => { u32 };
    (@T size D2) => { (u32,u32) };
    (@T size D3) => { (u32,u32,u32) };
    (@T size Cube) => { u32 };

    (@M dimensions D1) => {
        fn dimensions(&self) -> Dimensions {
            Dimensions::Dim1d { width: self.size, array_layers: self.array_layers }
        }
    };
    (@M dimensions D2) => {
        fn dimensions(&self) -> Dimensions {
            Dimensions::Dim2d { width: self.size.0, height: self.size.1, array_layers: self.array_layers }
        }
    };
    (@M dimensions D3) => {
        fn dimensions(&self) -> Dimensions {
            Dimensions::Dim3d { width: self.size.0, height: self.size.1, depth: self.size.2 }
        }
    };
    (@M dimensions Cube) => {
        fn dimensions(&self) -> Dimensions {
            Dimensions::DimCube { size: self.size, array_layers: self.array_layers }
        }
    };

    (@MM mipmap_methods) => {
        pub fn mipmaps(&mut self, mipmaps: MipmapsOption) -> &mut Self {
            self.mipmaps = mipmaps;
            self
        }
        pub fn no_mipmaps(&mut self) -> &mut Self {
            self.mipmaps(MipmapsOption::NoMipmap)
        }
        pub fn allocate_mipmaps(&mut self) -> &mut Self {
            self.mipmaps(MipmapsOption::Generate)
        }
        pub fn generate_mipmaps(&mut self) -> &mut Self {
            self.mipmaps(MipmapsOption::Generate)
        }
    };

    (@MM mipmap_methods D1 SS) => { impl_image_builder!(@MM mipmap_methods); };
    (@MM mipmap_methods D2 SS) => { impl_image_builder!(@MM mipmap_methods); };
    (@MM mipmap_methods D3 SS) => { impl_image_builder!(@MM mipmap_methods); };
    (@MM mipmap_methods Cube SS) => { impl_image_builder!(@MM mipmap_methods); };
    (@MM mipmap_methods $_:ident MS) => {};

    (@M samples MS) => {
        pub fn samples(&mut self, count: u32) -> &mut Self {
            self.samples = count;
            self
        }
    };

    (@M samples SS) => {};

    (@M array_layers) => {
        pub fn array_layers(&mut self, count: u32) -> &mut Self {
            self.array_layers = count;
            self
        }
    };

    (@M size D1) => {
        pub fn size(&mut self, w: u32) -> &mut Self {
            self.size = w;
            self
        }
    };

    (@M size D2) => {
        pub fn size(&mut self, w: u32, h: u32) -> &mut Self {
            self.size = (w,h).into();
            self
        }
    };

    (@M size Cube) => { impl_image_builder!(@M size D1) };

    (@M size D3) => {
        pub fn size(&mut self, w: u32, h: u32, d: u32) -> &mut Self {
            self.size = (w,h,d).into();
            self
        }
    };

    (@M array_layers D1) => { impl_image_builder!(@M array_layers); };
    (@M array_layers D2) => { impl_image_builder!(@M array_layers); };
    (@M array_layers Cube) => { impl_image_builder!(@M array_layers); };
    (@M array_layers D3) => { };

    (@E flags RW) => {
        ImageUsageFlags::COLOR_ATTACHMENT
            | ImageUsageFlags::INPUT_ATTACHMENT
            | ImageUsageFlags::STORAGE
            | ImageUsageFlags::SAMPLED
    };
    (@E flags RO) => { ImageUsageFlags::SAMPLED };
    (@E flags C) => { ImageUsageFlags::COLOR_ATTACHMENT };
    (@E flags DS) => { ImageUsageFlags::DEPTH_ATTACHMENT };

    (@M build) => {
        pub fn build(&mut self) -> O {
            let c = ImageCreateInfo {
                scope: self.aliasing,
                format: self.format,
                dimensions: self.dimensions(),
                mipmaps: self.mipmaps,
                samples: self.samples,
                usage: self.usage,
                data: None
            };
            (self.builder)(&c)
            /*$rty {
                image: self.arena.create_image(self.aliasing, self.format, self.dimensions(), self.mipmaps, self.samples, self.usage, None).image
            }*/
        }
    };

    // RO: Read-only (sample)
    // RW: Read/write (sample, storage, render target)
    // C: Color render target
    // DS: Depth stencil taret
    (@M build RW) => { impl_image_builder!(@M build); };
    (@M build C) => { impl_image_builder!(@M build); };
    (@M build DS) => { impl_image_builder!(@M build); };
    (@M build RO) => {};

    (@M with_data) => {
        pub fn with_data(&mut self, data: &[u8]) -> O {
            let c = ImageCreateInfo {
                scope: self.aliasing,
                format: self.format,
                dimensions: self.dimensions(),
                mipmaps: self.mipmaps,
                samples: self.samples,
                usage: self.usage,
                data: Some(data)
            };
            (self.builder)(&c)

            /*$rty {
                image: self.arena.create_image(self.aliasing, self.format, self.dimensions(), self.mipmaps, self.samples, self.usage, Some(data)).image
            }*/
        }
    };

    ($n:ident $mode:ident $shape:ident $multisample:ident) => {
        #[allow(dead_code)]
        pub struct $n<O, F: Fn(&ImageCreateInfo) -> O> {
            pub builder: F,
            pub format: Format,
            pub size: impl_image_builder!(@T size $shape),
            pub array_layers: u32,
            pub mipmaps: MipmapsOption,
            pub samples: u32,
            pub usage: ImageUsageFlags,
            pub aliasing: AliasScope,
        }

        impl<O, F: Fn(&ImageCreateInfo) -> O> $n<O,F> {
            pub fn new(format: Format, size: impl_image_builder!(@T size $shape), builder: F) -> $n<O,F> {
                $n {
                    builder,
                    format,
                    size,
                    array_layers: 1,
                    mipmaps: MipmapsOption::NoMipmap,
                    samples: 1,
                    usage: impl_image_builder!(@E flags $mode),
                    aliasing: AliasScope::no_alias(),
                }
            }

            impl_image_builder!(@M array_layers $shape);
            impl_image_builder!(@M dimensions $shape);
            impl_image_builder!(@M samples $multisample);
            impl_image_builder!(@M size $shape);
            impl_image_builder!(@MM mipmap_methods $shape $multisample);
            impl_image_builder!(@M build $mode);
            impl_image_builder!(@M with_data);
        }
    };
}

impl_image_builder!(Image1dBuilder RW D1 SS);
impl_image_builder!(Image2dBuilder RW D2 MS);
impl_image_builder!(Image3dBuilder RW D3 SS);
impl_image_builder!(RenderTargetBuilder       C  D2 MS);
impl_image_builder!(DepthStencilTargetBuilder DS D2 MS);

//--------------------------------------------------------------------------------------------------
// Image types

fn normalize_mip_range(miprange: impl RangeBounds<u32>) -> (u32, Option<u32>) {
    let a = match miprange.start_bound() {
        Bound::Unbounded => 0,
        Bound::Excluded(&n) => n + 1,
        Bound::Included(&n) => n,
    };
    let b = match miprange.start_bound() {
        Bound::Unbounded => None,
        Bound::Excluded(&n) => Some(n),
        Bound::Included(&n) => Some(n + 1),
    };
    (a, b)
}

macro_rules! impl_image {
    ($n:ident) => {
        #[derive(derivative::Derivative)]
        #[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
        pub struct $n<'a, B: Backend> {
            pub(crate) image: &'a B::Image,
        }

        impl<'a, B: Backend> $n<'a, B> {
            pub fn inner(&self) -> &'a B::Image {
                &self.image
            }

            pub unsafe fn from_raw(raw: &'a B::Image) -> $n<'a, B> {
                $n { image: raw }
            }
        }
    };
}

macro_rules! impl_image_mipmap {
    ($n_image:ident, $n_image_mipmap:ident) => {
        #[derive(derivative::Derivative)]
        #[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
        pub struct $n_image_mipmap<'a, B: Backend> {
            pub(crate) image: &'a B::Image,
            pub(crate) miplevel: u32,
        }

        impl<'a, B: Backend> $n_image<'a, B> {
            pub fn mipmap(&self, miplevel: u32) -> $n_image_mipmap<'a, B> {
                $n_image_mipmap {
                    image: self.image,
                    miplevel,
                }
            }
        }
    };
}

macro_rules! impl_image_mipmaps {
    ($n_image:ident, $n_image_mipmaps:ident, $texture_sampler_view:ident) => {
        #[derive(derivative::Derivative)]
        #[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
        pub struct $n_image_mipmaps<'a, B: Backend> {
            pub(crate) image: &'a B::Image,
            pub(crate) most_detailed_miplevel: u32,
            pub(crate) mip_count: Option<u32>,
        }

        impl<'a, B: Backend> $n_image_mipmaps<'a, B> {
            pub fn sampled(&self, sampler: SamplerDescription) -> $texture_sampler_view<'a, B> {
                $texture_sampler_view {
                    image: self.image,
                    sampler,
                    subresource: SubresourceRange {
                        base_mip_level: self.most_detailed_miplevel,
                        level_count: self.mip_count,
                        base_array_layer: 0,
                        layer_count: Some(1),
                    },
                }
            }
            pub fn sampled_linear(&self) -> $texture_sampler_view<'a, B> {
                self.sampled(SamplerDescription::LINEAR_MIPMAP_LINEAR)
            }
            pub fn sampled_nearest(&self) -> $texture_sampler_view<'a, B> {
                self.sampled(SamplerDescription::NEAREST_MIPMAP_NEAREST)
            }
        }

        impl<'a, B: Backend> $n_image<'a, B> {
            pub fn mipmaps(&self, miprange: impl RangeBounds<u32>) -> $n_image_mipmaps<'a, B> {
                let (most_detailed_miplevel, mip_count) = normalize_mip_range(miprange);
                $n_image_mipmaps {
                    image: self.image,
                    most_detailed_miplevel,
                    mip_count,
                }
            }
            pub fn sampled(&self, sampler: SamplerDescription) -> $texture_sampler_view<'a, B> {
                self.mipmaps(0..).sampled(sampler)
            }
            pub fn sampled_linear(&self) -> $texture_sampler_view<'a, B> {
                self.mipmaps(0..)
                    .sampled(SamplerDescription::LINEAR_MIPMAP_LINEAR)
            }
            pub fn sampled_nearest(&self) -> $texture_sampler_view<'a, B> {
                self.mipmaps(0..)
                    .sampled(SamplerDescription::NEAREST_MIPMAP_NEAREST)
            }
        }
    };
}

impl_image!(UnsafeImage);
impl_image!(Image1d);
impl_image_mipmap!(Image1d, Image1dMipmap);
impl_image_mipmaps!(Image1d, Image1dMipmaps, TextureSampler1dView);
impl_image!(Image2d);
impl_image_mipmap!(Image2d, Image2dMipmap);
impl_image_mipmaps!(Image2d, Image2dMipmaps, TextureSampler2dView);
impl_image!(Image3d);
impl_image_mipmap!(Image3d, Image3dMipmap);
impl_image_mipmaps!(Image3d, Image3dMipmaps, TextureSampler3dView);
impl_image!(RenderTargetImage2d);
impl_image!(DepthStencilImage2d);

//pub struct RenderTargetImage<'a, B: Backend>(pub(crate) &'a B::Image);
//pub struct DepthStencilImage<'a, B: Backend>(pub(crate) &'a B::Image);

impl<'a, B: Backend> RenderTargetImage2d<'a, B> {
    pub fn render_target_view(&self) -> RenderTarget2dView<'a, B> {
        RenderTarget2dView {
            image: self.image,
            subresource: SubresourceRange {
                base_mip_level: 0,
                level_count: Some(1),
                base_array_layer: 0,
                layer_count: Some(1),
            },
        }
    }
}

impl<'a, B: Backend> Image2d<'a, B> {
    pub fn render_target_view(&self) -> RenderTarget2dView<'a, B> {
        RenderTarget2dView {
            image: self.image,
            subresource: SubresourceRange {
                base_mip_level: 0,
                level_count: Some(1),
                base_array_layer: 0,
                layer_count: Some(1),
            },
        }
    }
}

impl<'a, B: Backend> Image2dMipmap<'a, B> {
    pub fn render_target_view(&self) -> RenderTarget2dView<'a, B> {
        RenderTarget2dView {
            image: self.image,
            subresource: SubresourceRange {
                base_mip_level: self.miplevel,
                level_count: Some(1),
                base_array_layer: 0,
                layer_count: Some(1),
            },
        }
    }
}

//--------------------------------------------------------------------------------------------------
macro_rules! impl_view_type {
    ($nv:ident) => {
        impl_view_type!($nv from);
    };
    (sampled $nv:ident) => {
        impl_view_type!(sampled $nv from);
    };
    ($nv:ident from $($trivial_conv:ident),*) => {
        #[derive(derivative::Derivative)]
        #[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
        pub struct $nv<'a, B: Backend> {
            pub(crate) image: &'a B::Image,
            pub(crate) subresource: SubresourceRange,
        }
        impl<'a, B: Backend> $nv<'a,B> {
            pub fn inner(&self) -> &'a B::Image { self.image }
            pub fn subresource(&self) -> SubresourceRange { self.subresource }
        }
        $(impl<'a,B:Backend> From<$trivial_conv<'a,B>> for $nv<'a,B> {
            fn from(other: $trivial_conv<'a,B>) -> $nv<'a,B> {
                $nv {
                    image: other.image,
                    subresource: other.subresource,
                }
            }
        })*
    };
    (sampled $nv:ident from $($trivial_conv:ident),*) => {
        #[derive(derivative::Derivative)]
        #[derivative(Copy(bound = ""), Clone(bound = ""), Debug(bound = ""))]
        pub struct $nv<'a, B: Backend> {
            pub(crate) image: &'a B::Image,
            pub(crate) subresource: SubresourceRange,
            pub(crate) sampler: SamplerDescription,
        }
        impl<'a, B: Backend> $nv<'a,B> {
            pub fn inner(&self) -> &'a B::Image { self.image }
            pub fn subresource(&self) -> SubresourceRange { self.subresource }
            pub fn sampler(&self) -> &SamplerDescription { &self.sampler }
        }
        $(impl<'a,B:Backend> From<$trivial_conv<'a,B>> for $nv<'a,B> {
            fn from(other: $trivial_conv<'a,B>) -> $nv<'a,B> {
                $nv {
                    image: other.image,
                    subresource: other.subresource,
                    sampler: other.sampler,
                }
            }
        })*
    };
}

macro_rules! impl_resource_interface_view {
    ($nv: ident, $rt:expr, $desc:ident) => {
        impl<'a, B: Backend> ResourceInterface<'a, B> for $nv<'a, B> {
            const TYPE: ResourceBindingType = $rt;
            const DATA_TYPE: Option<&'static TypeDesc<'static>> = None;
            fn into_descriptor(self) -> Descriptor<'a, B> {
                Descriptor::$desc {
                    image: self.image,
                    subresource: self.subresource,
                }
            }
        }
    };

    (sampled $nv: ident, $rt:expr, $desc:ident) => {
        impl<'a, B: Backend> ResourceInterface<'a, B> for $nv<'a, B> {
            const TYPE: ResourceBindingType = $rt;
            const DATA_TYPE: Option<&'static TypeDesc<'static>> = None;
            fn into_descriptor(self) -> Descriptor<'a, B> {
                Descriptor::$desc {
                    image: self.image,
                    subresource: self.subresource,
                    sampler: self.sampler,
                }
            }
        }
    };
}

macro_rules! impl_single_mipmap_view {
    (default $n:ident => $nv:ident) => {
        impl<'a, B: Backend> From<$n<'a, B>> for $nv<'a, B> {
            fn from(other: $n<'a, B>) -> $nv<'a, B> {
                $nv {
                    image: other.image,
                    subresource: SubresourceRange {
                        base_mip_level: 0,
                        level_count: Some(1),
                        base_array_layer: 0,
                        layer_count: Some(1),
                    },
                }
            }
        }
    };
    ($n:ident => $nv:ident) => {
        impl<'a, B: Backend> From<$n<'a, B>> for $nv<'a, B> {
            fn from(other: $n<'a, B>) -> $nv<'a, B> {
                $nv {
                    image: other.image,
                    subresource: SubresourceRange {
                        base_mip_level: other.miplevel,
                        level_count: Some(1),
                        base_array_layer: 0,
                        layer_count: Some(1),
                    },
                }
            }
        }
    };
}

// also, any image can be converted to an imageview
impl_view_type!(ImageView from Image1dView,Image2dView,Image3dView);
impl_view_type!(Image1dView);
impl_view_type!(Image2dView);
impl_view_type!(Image3dView);
impl_single_mipmap_view!(default Image1d => Image1dView);
impl_single_mipmap_view!(default Image2d => Image2dView);
impl_single_mipmap_view!(default Image3d => Image3dView);
impl_single_mipmap_view!(default RenderTargetImage2d => Image2dView);
impl_single_mipmap_view!(default DepthStencilImage2d => Image2dView);
impl_single_mipmap_view!(Image1dMipmap => Image1dView);
impl_single_mipmap_view!(Image2dMipmap => Image2dView);
impl_single_mipmap_view!(Image3dMipmap => Image3dView);

impl_view_type!(RenderTargetView from RenderTarget2dView);
impl_view_type!(DepthStencilView from DepthStencil2dView);
impl_view_type!(RenderTarget2dView);
impl_view_type!(DepthStencil2dView);
// img2d, default level can be converted to RTV via into
impl_single_mipmap_view!(default Image2d => RenderTargetView);
impl_single_mipmap_view!(default Image2d => RenderTarget2dView);
// rtimage2d can be converted to RTV via into
impl_single_mipmap_view!(default RenderTargetImage2d => RenderTargetView);
impl_single_mipmap_view!(default RenderTargetImage2d => RenderTarget2dView);
// img2dmipmap can be converted to RTV via into
impl_single_mipmap_view!(Image2dMipmap => RenderTargetView);
impl_single_mipmap_view!(Image2dMipmap => RenderTarget2dView);

impl_view_type!(RwImage1dView);
impl_view_type!(RwImage2dView);
impl_view_type!(RwImage3dView);
// imgNd can be converted to RwImage via into
impl_single_mipmap_view!(default Image1d => RwImage1dView);
impl_single_mipmap_view!(default Image2d => RwImage2dView);
impl_single_mipmap_view!(default Image3d => RwImage3dView);
impl_single_mipmap_view!(Image1dMipmap => RwImage1dView);
impl_single_mipmap_view!(Image2dMipmap => RwImage2dView);
impl_single_mipmap_view!(Image3dMipmap => RwImage3dView);

impl_resource_interface_view!(
    RwImage1dView,
    ResourceBindingType::RwImage(ResourceShape::R1d),
    RwImage
);
impl_resource_interface_view!(
    RwImage2dView,
    ResourceBindingType::RwImage(ResourceShape::R2d),
    RwImage
);
impl_resource_interface_view!(
    RwImage3dView,
    ResourceBindingType::RwImage(ResourceShape::R3d),
    RwImage
);

impl_view_type!(sampled TextureSamplerView from TextureSampler1dView,TextureSampler2dView,TextureSampler3dView);
impl_view_type!(sampled TextureSampler1dView);
impl_view_type!(sampled TextureSampler2dView);
impl_view_type!(sampled TextureSampler3dView);

impl_resource_interface_view!(sampled TextureSampler1dView, ResourceBindingType::TextureSampler(ResourceShape::R1d), TextureSampler);
impl_resource_interface_view!(sampled TextureSampler2dView, ResourceBindingType::TextureSampler(ResourceShape::R2d), TextureSampler);
impl_resource_interface_view!(sampled TextureSampler3dView, ResourceBindingType::TextureSampler(ResourceShape::R3d), TextureSampler);
