use crate::{api as gl, api::types::*};
use gfx2::{Format, FormatInfo};

pub struct GlFormatInfo {
    pub internal_fmt: GLenum,
    pub upload_components: GLenum, //< Matching external format for uploads/reads (so that OpenGL does not have to do any conversion)
    pub upload_ty: GLenum,         //< Matching element type for uploads/reads
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
    pub fn from_format(fmt: Format) -> &'static GlFormatInfo {
        match fmt {
            Format::R8_UNORM => &GLF_R8_UNORM,
            Format::R8_SNORM => &GLF_R8_SNORM,
            Format::R8_UINT => &GLF_R8_UINT,
            Format::R8_SINT => &GLF_R8_SINT,
            Format::R16G16_SFLOAT => &GLF_R16G16_SFLOAT,
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
