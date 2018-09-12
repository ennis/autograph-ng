use bitflags::*;
use format::Format;
use std::cmp::max;

/// The dimensions of a texture.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TextureDimensions {
    Tex1D,
    Tex2D,
    Tex3D,
    Tex1DArray,
    Tex2DArray,
    TexCube,
}

bitflags! {
    #[derive(Default)]
    pub struct TextureOptions: u8 {
        ///
        const SPARSE_STORAGE = 0b00000001;
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MipMaps {
    Auto,
    Count(u8),
}

//2+2+4+4+4+2+2+1 = 21 bytes
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TextureDesc {
    /// Texture dimensions.
    pub dimensions: TextureDimensions,
    /// Texture storage format.
    pub format: Format,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels, or array size of 1D texture arrays.
    pub height: u32,
    /// Depth in pixels, or array size of 2D texture arrays.
    pub depth: u32,
    /// Number of samples for multisample textures.
    /// 0 means that the texture will not be allocated with multisampling.
    pub sample_count: u8,
    /// Number of mipmap levels that should be allocated for this texture.
    /// See also: `get_texture_mip_map_count`
    pub mip_map_count: MipMaps,
    ///
    pub options: TextureOptions,
}

impl Default for TextureDesc {
    fn default() -> TextureDesc {
        TextureDesc {
            dimensions: TextureDimensions::Tex2D,
            format: Format::R8G8B8A8_UNORM,
            width: 0,
            height: 0,
            depth: 0,
            sample_count: 0,
            mip_map_count: MipMaps::Count(1),
            options: TextureOptions::empty(),
        }
    }
}

/// Wrapper for OpenGL textures.
///
/// To create a `Texture` object, use the constructor with a `TextureDesc`
/// object describing the texture.
/// The underlying GL texture object is created with immutable storage, meaning
/// that it is impossible to reallocate the storage (resizing, adding mip
/// levels) once the texture is created.
///
/// The texture object is bound to the context lifetime. It is checked dynamically.
/// Raw OpenGL textures are not meant to be used directly: instead,
/// use the interface provided by `Context` and `Frame` to create and manage
/// persistent and transient resources.
#[derive(Debug)]
pub(crate) struct TextureObject {
    //pub(crate) obj: GLuint,
    pub(crate) desc: TextureDesc,
}

/// Trait for pixel types that can be uploaded to the GPU with glTextureSubImage*.
/// Describes the format of the client data.
pub trait ClientFormatInfo {
    //fn get_format_info() -> FormatInfo;
}

/// impl for (f32xN) tuples
/// impl for (u8xN) tuples

impl TextureObject {
    /// Returns the TextureDesc object describing this texture
    pub fn desc(&self) -> &TextureDesc {
        &self.desc
    }

    /// The width in pixels of this texture
    pub fn width(&self) -> u32 {
        self.desc.width
    }

    /// The height in pixels of this texture or the array size of 1D textures
    /// Returns 1 if it is a 1D texture
    pub fn height(&self) -> u32 {
        self.desc.height
    }

    /// The depth in pixels of this texture for 3D textures or the array size of 2D texture arrays
    pub fn depth(&self) -> u32 {
        self.desc.depth
    }

    /// Create a new texture object based on the given description
    pub fn new(desc: &TextureDesc) -> TextureObject {
        /*let target = match desc.dimensions {
            TextureDimensions::Tex1D => gl::TEXTURE_1D,
            TextureDimensions::Tex2D => if desc.sample_count > 1 {
                gl::TEXTURE_2D_MULTISAMPLE
            } else {
                gl::TEXTURE_2D
            },
            TextureDimensions::Tex3D => gl::TEXTURE_3D,
            _ => unimplemented!("texture type"),
        };

        let glfmt = GlFormatInfo::from_format(desc.format);
        let mut obj = 0;
        let mip_map_count = match desc.mip_map_count {
            MipMaps::Auto => get_texture_mip_map_count(desc.width, desc.height),
            MipMaps::Count(count) => {
                // Multisampled textures can't have more than one mip level
                assert!(desc.sample_count <= 1 || count == 1);
                count
            }
        };

        unsafe {
            gl::CreateTextures(target, 1, &mut obj);

            if desc.options.contains(SPARSE_STORAGE) {
                gl::TextureParameteri(obj, gl::TEXTURE_SPARSE_ARB, gl::TRUE as i32);
            }

            match target {
                gl::TEXTURE_1D => {
                    gl::TextureStorage1D(
                        obj,
                        mip_map_count as i32,
                        glfmt.internal_fmt,
                        desc.width as i32,
                    );
                }
                gl::TEXTURE_2D => {
                    gl::TextureStorage2D(
                        obj,
                        mip_map_count as i32,
                        glfmt.internal_fmt,
                        desc.width as i32,
                        desc.height as i32,
                    );
                }
                gl::TEXTURE_2D_MULTISAMPLE => {
                    gl::TextureStorage2DMultisample(
                        obj,
                        desc.sample_count as i32,
                        glfmt.internal_fmt,
                        desc.width as i32,
                        desc.height as i32,
                        true as u8,
                    );
                }
                gl::TEXTURE_3D => {
                    gl::TextureStorage3D(
                        obj,
                        1,
                        glfmt.internal_fmt,
                        desc.width as i32,
                        desc.height as i32,
                        desc.depth as i32,
                    );
                }
                _ => unimplemented!("texture type"),
            };

            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }

        TextureObject {
            desc: desc.clone(),
            obj,
        }*/
        unimplemented!()
    }

    /// Texture upload
    /// Ideally, the texture format should be embedded into the texture type
    /// This function will simply take a raw slice and upload it
    /// It will, however, check that the slice has the correct size
    ///
    /// TextureFormats include both size & type information, and how to interpret the data (channels)
    /// ClientPixelData must return: the size of the data
    ///
    pub fn upload_region(
        &mut self,
        mip_level: i32,
        offset: (u32, u32, u32),
        size: (u32, u32, u32),
        data: &[u8],
    ) {
        /*let fmtinfo = self.desc.format.get_format_info();
        assert!(
            !fmtinfo.is_compressed(),
            "Compressed image data upload is not yet supported"
        );
        assert!(
            data.len() == (size.0 * size.1 * size.2) as usize * fmtinfo.byte_size(),
            "image data size mismatch"
        );
        // TODO check size of mip level
        let glfmt = GlFormatInfo::from_format(self.desc.format);

        let mut prev_unpack_alignment = 0;
        unsafe {
            gl::GetIntegerv(gl::UNPACK_ALIGNMENT, &mut prev_unpack_alignment);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        };

        match self.desc.dimensions {
            TextureDimensions::Tex1D => unsafe {
                gl::TextureSubImage1D(
                    self.obj,
                    mip_level,
                    offset.0 as i32,
                    size.0 as i32,
                    glfmt.upload_components,
                    glfmt.upload_ty,
                    data.as_ptr() as *const GLvoid,
                );
            },
            TextureDimensions::Tex2D => unsafe {
                gl::TextureSubImage2D(
                    self.obj,
                    mip_level,
                    offset.0 as i32,
                    offset.1 as i32,
                    size.0 as i32,
                    size.1 as i32,
                    glfmt.upload_components,
                    glfmt.upload_ty,
                    data.as_ptr() as *const GLvoid,
                );
            },
            TextureDimensions::Tex3D => unsafe {
                gl::TextureSubImage3D(
                    self.obj,
                    mip_level,
                    offset.0 as i32,
                    offset.1 as i32,
                    offset.2 as i32,
                    size.0 as i32,
                    size.1 as i32,
                    size.2 as i32,
                    glfmt.upload_components,
                    glfmt.upload_ty,
                    data.as_ptr() as *const GLvoid,
                );
            },
            _ => unimplemented!("Unsupported image upload"),
        };

        unsafe {
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, prev_unpack_alignment);
        }

        /*let channels = match fmtinfo.component_layout {
            ComponentLayout::RGBA => gl::RGBA,
            ComponentLayout::R => gl::RED,
            ComponentLayout::RGB => gl::RGB,
            ComponentLayout::RG => gl::RG,
            ComponentLayout::BGR => gl::BGR,
            ComponentLayout::BGRA => gl::BGRA,
            ComponentLayout::D => gl::DEPTH_COMPONENT,
            ComponentLayout::S => gl::STENCIL_INDEX,
            _ => panic!("Unsupported component layout for client pixel data")
        };

        if fmtinfo.is_compressed() {
            panic!();
        }


        let is_simple_integral_format = |fmtinfo| match fmtinfo.format_type
            {
                UNKNOWN => panic!("Unknown component type"),
                UNORM | SNORM | USCALED | SSCALED | UINT | SINT | SRGB => true,
                UFLOAT | SFLOAT => false,
                UNORM_UINT | SFLOAT_UINT => false,
                _ => panic!("Unsupoorted component type")
            };

        let component_ty = match fmtinfo.component_bits {
            [8,0,0,0] | [8,8,0,0] | [8,8,8,0] =>
                match fmtinfo.format_type {
                    UNORM | USCALED | UINT | SRGB => gl::UNSIGNED_BYTE,
                    SNORM | SSCALED | SINT => gl::BYTE,
                    _ => panic!("unexpected format type")
                },
            [8,8,8,8] => match fmtinfo.format_type {
                UNORM | USCALED | UINT | SRGB => gl::UNSIGNED_INT_8_8_8_8,
                SNORM | SSCALED | SINT => gl::BYTE,
                _ => panic!("unexpected format type")
            },
            [16,0,0,0] | [16,16,0,0] | [16,16,16,0] | [16,16,16,16] => match fmtinfo.format_type {
                UNORM | USCALED | UINT => gl::UNSIGNED_SHORT,
                SNORM | SSCALED | SINT => gl::SHORT,
                _ => panic!("unexpected format type")
            },
            [32,0,0,0] | [32,32,0,0] | [32,32,32,0] | [32,32,32,32] => match fmtinfo.format_type {
                UNORM | USCALED | UINT => gl::UNSIGNED_INT,
                SNORM | SSCALED | SINT => gl::INT,
                UFLOAT | SFLOAT => gl::FLOAT,
                _ => panic!("unsupported format type")
            },
            [2, 10, 10, 10] => match fmtinfo.format_type {
                UNORM | USCALED | UINT => gl::UNSIGNED_INT_2_10_10_10_REV,
                SNORM | SSCALED | SINT => gl::UNSIGNED_INT_2_10_10_10_REV,  // XXX opengl does not support signed version
                _ => panic!("unsupported format type")
            },
        };

        let signedness = match fmtinfo.format_type
            {
                UNKNOWN => panic!("Unknown pixel format"),
                UNORM => false,
                SNORM => true,
                USCALED => false,
                SSCALED => true,
                UINT => false,
                SINT => true,
                SRGB => ,
                UFLOAT,
                SFLOAT,
                UNORM_UINT,
                SFLOAT_UINT

            }*/*/
        unimplemented!()
    }

    /*pub fn gl_object(&self) -> GLuint {
        self.obj
    }*/
}

impl Drop for TextureObject {
    fn drop(&mut self) {
        unimplemented!()
        /*unsafe {
            gl::DeleteTextures(1, &self.obj);
        }*/
    }
}

///
/// Get the maximum number of mip map levels for a 2D texture of size (width,height)
/// numLevels = 1 + floor(log2(max(w, h, d)))
///
/// # References
///
/// https://stackoverflow.com/questions/9572414/how-many-mipmaps-does-a-texture-have-in-opengl
pub fn get_texture_mip_map_count(width: u32, height: u32) -> u8 {
    1 + f32::floor(f32::log2(max(width, height) as f32)) as u8
}
