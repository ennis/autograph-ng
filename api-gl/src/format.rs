use crate::{api as gl, api::types::*};
use autograph_api::Format;

/// Equivalent OpenGL format information for a given [Format](autograph_api::Format).
pub struct GlFormatInfo {
    /// Corresponding internal format.
    pub internal_fmt: GLenum,
    /// Matching external format for uploads/reads (so that OpenGL does not have to do any conversion).
    pub upload_components: GLenum,
    /// Matching element type for uploads/reads.
    pub upload_ty: GLenum,
}

static GLF_R8_UNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R8,
    upload_components: gl::RED,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_R8_SNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R8_SNORM,
    upload_components: gl::RED,
    upload_ty: gl::BYTE,
};
static GLF_R8_UINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R8UI,
    upload_components: gl::RED,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_R8_SINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R8I,
    upload_components: gl::RED,
    upload_ty: gl::BYTE,
};

static GLF_R16_UNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R16,
    upload_components: gl::RED,
    upload_ty: gl::UNSIGNED_SHORT,
};
static GLF_R16_SNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R16_SNORM,
    upload_components: gl::RED,
    upload_ty: gl::SHORT,
};
static GLF_R16_UINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R16UI,
    upload_components: gl::RED,
    upload_ty: gl::UNSIGNED_SHORT,
};
static GLF_R16_SINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R16I,
    upload_components: gl::RED,
    upload_ty: gl::SHORT,
};

static GLF_R32_SINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R32I,
    upload_components: gl::RED,
    upload_ty: gl::INT,
};
static GLF_R32_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R32F,
    upload_components: gl::RED,
    upload_ty: gl::FLOAT,
};
static GLF_R16_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::R16F,
    upload_components: gl::RED,
    upload_ty: gl::FLOAT,
}; // XXX no half-float for upload!
static GLF_R16G16_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RG16F,
    upload_components: gl::RG,
    upload_ty: gl::FLOAT,
}; // XXX no half-float for upload!
static GLF_R16G16B16A16_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA16F,
    upload_components: gl::RGBA,
    upload_ty: gl::FLOAT,
}; // XXX no half-float for upload!
static GLF_R32G32_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RG32F,
    upload_components: gl::RG,
    upload_ty: gl::FLOAT,
};
static GLF_R32G32B32_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGB32F,
    upload_components: gl::RGB,
    upload_ty: gl::FLOAT,
};
static GLF_R32G32B32A32_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA32F,
    upload_components: gl::RGBA,
    upload_ty: gl::FLOAT,
};
static GLF_R8G8B8A8_UNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA8,
    upload_components: gl::RGBA,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_R8G8B8A8_SNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA8_SNORM,
    upload_components: gl::RGBA,
    upload_ty: gl::BYTE,
};
static GLF_R16G16B16_UNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGB16,
    upload_components: gl::RGB,
    upload_ty: gl::UNSIGNED_SHORT,
};
static GLF_R16G16B16A16_UNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA16,
    upload_components: gl::RGBA,
    upload_ty: gl::UNSIGNED_SHORT,
};
static GLF_R16G16B16_SNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGB16_SNORM,
    upload_components: gl::RGB,
    upload_ty: gl::SHORT,
};
static GLF_R16G16B16A16_SNORM: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA16_SNORM,
    upload_components: gl::RGBA,
    upload_ty: gl::SHORT,
};
static GLF_R16G16B16A16_UINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA16UI,
    upload_components: gl::RGBA,
    upload_ty: gl::UNSIGNED_SHORT,
};
static GLF_R16G16B16A16_SINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA16I,
    upload_components: gl::RGBA,
    upload_ty: gl::SHORT,
};
static GLF_R8G8B8A8_UINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA8UI,
    upload_components: gl::RGBA,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_R8G8B8A8_SINT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::RGBA8I,
    upload_components: gl::RGBA,
    upload_ty: gl::BYTE,
};
static GLF_R8G8B8_SRGB: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::SRGB8,
    upload_components: gl::RGB,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_R8G8B8A8_SRGB: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::SRGB8_ALPHA8,
    upload_components: gl::RGBA,
    upload_ty: gl::UNSIGNED_BYTE,
};
static GLF_D32_SFLOAT: GlFormatInfo = GlFormatInfo {
    internal_fmt: gl::DEPTH_COMPONENT32F,
    upload_components: gl::DEPTH_COMPONENT,
    upload_ty: gl::FLOAT,
};

impl GlFormatInfo {
    /// Returns the equivalent OpenGL format information for the specified format.
    pub fn from_format(fmt: Format) -> &'static GlFormatInfo {
        match fmt {
            Format::R8_UNORM => &GLF_R8_UNORM,
            Format::R8_SNORM => &GLF_R8_SNORM,
            Format::R8_UINT => &GLF_R8_UINT,
            Format::R8_SINT => &GLF_R8_SINT,

            Format::R16_UNORM => &GLF_R16_UNORM,
            Format::R16_SNORM => &GLF_R16_SNORM,
            Format::R16_UINT => &GLF_R16_UINT,
            Format::R16_SINT => &GLF_R16_SINT,

            Format::R32_SINT => &GLF_R32_SINT,
            Format::R32_SFLOAT => &GLF_R32_SFLOAT,
            Format::R16_SFLOAT => &GLF_R16_SFLOAT,
            Format::R16G16_SFLOAT => &GLF_R16G16_SFLOAT,
            Format::R16G16B16_SNORM => &GLF_R16G16B16_SNORM,
            Format::R16G16B16_UNORM => &GLF_R16G16B16_UNORM,
            Format::R16G16B16A16_SNORM => &GLF_R16G16B16A16_SNORM,
            Format::R16G16B16A16_UNORM => &GLF_R16G16B16A16_UNORM,
            Format::R16G16B16A16_SINT => &GLF_R16G16B16A16_SINT,
            Format::R16G16B16A16_UINT => &GLF_R16G16B16A16_UINT,
            Format::R16G16B16A16_SFLOAT => &GLF_R16G16B16A16_SFLOAT,
            Format::R32G32_SFLOAT => &GLF_R32G32_SFLOAT,
            Format::R32G32B32_SFLOAT => &GLF_R32G32B32_SFLOAT,
            Format::R32G32B32A32_SFLOAT => &GLF_R32G32B32A32_SFLOAT,
            Format::R8G8B8A8_UNORM => &GLF_R8G8B8A8_UNORM,
            Format::R8G8B8A8_SNORM => &GLF_R8G8B8A8_SNORM,
            Format::R8G8B8A8_UINT => &GLF_R8G8B8A8_UINT,
            Format::R8G8B8A8_SINT => &GLF_R8G8B8A8_SINT,
            Format::R8G8B8_SRGB => &GLF_R8G8B8_SRGB,
            Format::R8G8B8A8_SRGB => &GLF_R8G8B8A8_SRGB,
            Format::D32_SFLOAT => &GLF_D32_SFLOAT,
            _ => panic!("Unsupported format: {:?}", fmt),
        }
    }
}
