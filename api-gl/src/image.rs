use crate::{
    api as gl,
    api::{types::*, Gl},
    format::GlFormatInfo,
    AliasInfo,
};
use autograph_api::{
    get_texture_mip_map_count, Dimensions, Format, ImageUsageFlags, MipmapsOption,
};
use slotmap::new_key_type;
use std::cmp::max;

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ImageDescription {
    pub format: Format,
    pub dimensions: Dimensions,
    pub mipcount: u32,
    pub samples: u32,
    pub usage: ImageUsageFlags,
}

impl ImageDescription {
    pub fn new(
        format: Format,
        dimensions: Dimensions,
        mipmaps: MipmapsOption,
        samples: u32,
        usage: ImageUsageFlags,
    ) -> ImageDescription {
        let (w, h, _d) = dimensions.width_height_depth();
        let mipcount = match mipmaps {
            // TODO mipcount for 3D textures?
            MipmapsOption::Allocate | MipmapsOption::Generate => {
                get_texture_mip_map_count(max(w, h))
            }
            MipmapsOption::GenerateCount(n) | MipmapsOption::AllocateCount(n) => {
                if samples > 1 {
                    assert_eq!(n, 1);
                }
                n
            }
            MipmapsOption::NoMipmap => 1,
        };
        ImageDescription {
            format,
            dimensions,
            mipcount,
            usage,
            samples,
        }
    }
}

//--------------------------------------------------------------------------------------------------
struct ExtentsAndType {
    target: GLenum,
    width: u32,
    height: u32,
    depth: u32,
    array_layers: u32,
}

impl ExtentsAndType {
    fn from_dimensions(dim: &Dimensions) -> ExtentsAndType {
        match *dim {
            Dimensions::Dim1d {
                width,
                array_layers,
            } => ExtentsAndType {
                target: gl::TEXTURE_1D,
                width,
                height: 1,
                depth: 1,
                array_layers,
            },
            Dimensions::Dim2d {
                width,
                height,
                array_layers,
            } => ExtentsAndType {
                target: gl::TEXTURE_2D,
                width,
                height,
                depth: 1,
                array_layers,
            },
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => ExtentsAndType {
                target: gl::TEXTURE_3D,
                width,
                height,
                depth,
                array_layers: 1,
            },
            _ => unimplemented!(),
        }
    }
}

//--------------------------------------------------------------------------------------------------

/// Wrapper for OpenGL textures and renderbuffers.
#[derive(Copy, Clone, Debug)]
pub struct RawImage {
    pub(crate) obj: GLuint,
    pub(crate) target: GLenum,
    //pub format: Format,
}

impl RawImage {
    pub fn new(gl: &Gl, d: &ImageDescription) -> RawImage {
        if d.usage != ImageUsageFlags::COLOR_ATTACHMENT {
            // will be used as storage or sampled image
            RawImage::new_texture(gl, d.format, &d.dimensions, d.mipcount, d.samples)
        } else {
            // only used as color attachments: can use a renderbuffer instead
            RawImage::new_renderbuffer(gl, d.format, &d.dimensions, d.samples)
        }
    }

    pub fn new_texture(
        gl: &Gl,
        format: Format,
        dimensions: &Dimensions,
        mipcount: u32,
        samples: u32,
    ) -> RawImage {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = GlFormatInfo::from_format(format);

        if et.array_layers > 1 {
            unimplemented!("array textures")
        }

        let mut obj = 0;
        unsafe {
            gl.CreateTextures(et.target, 1, &mut obj);

            match et.target {
                gl::TEXTURE_1D => {
                    gl.TextureStorage1D(obj, mipcount as i32, glfmt.internal_fmt, et.width as i32);
                }
                gl::TEXTURE_2D => {
                    if samples > 1 {
                        gl.TextureStorage2DMultisample(
                            obj,
                            samples as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                            true as u8,
                        );
                    } else {
                        gl.TextureStorage2D(
                            obj,
                            mipcount as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                        );
                    }
                }
                gl::TEXTURE_3D => {
                    gl.TextureStorage3D(
                        obj,
                        1,
                        glfmt.internal_fmt,
                        et.width as i32,
                        et.height as i32,
                        et.depth as i32,
                    );
                }
                _ => unimplemented!("texture type"),
            };

            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }

        RawImage {
            obj,
            target: et.target,
            //format
        }
    }

    pub fn new_renderbuffer(
        gl: &Gl,
        format: Format,
        dimensions: &Dimensions,
        samples: u32,
    ) -> RawImage {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = GlFormatInfo::from_format(format);

        let mut obj = 0;

        unsafe {
            gl.CreateRenderbuffers(1, &mut obj);

            if samples > 1 {
                gl.NamedRenderbufferStorageMultisample(
                    obj,
                    samples as i32,
                    glfmt.internal_fmt,
                    et.width as i32,
                    et.height as i32,
                );
            } else {
                gl.NamedRenderbufferStorage(
                    obj,
                    glfmt.internal_fmt,
                    et.width as i32,
                    et.height as i32,
                );
            }
        }

        RawImage {
            obj,
            target: gl::RENDERBUFFER,
            //format
        }
    }

    /*pub fn is_renderbuffer(&self) -> bool {
        self.target == gl::RENDERBUFFER
    }*/

    pub fn destroy(&self, gl: &Gl) {
        unsafe {
            if self.target == gl::RENDERBUFFER {
                gl.DeleteRenderbuffers(1, &self.obj);
            } else {
                gl.DeleteTextures(1, &self.obj);
            }
        }
    }
}

/// Texture upload
///
/// TODO move in cmd
pub unsafe fn upload_image_region(
    gl: &Gl,
    target: GLenum,
    img: GLuint,
    fmt: Format,
    mip_level: i32,
    offset: (u32, u32, u32),
    size: (u32, u32, u32),
    data: &[u8],
) {
    let fmtinfo = fmt.get_format_info();
    assert_eq!(
        data.len(),
        (size.0 * size.1 * size.2) as usize * fmtinfo.byte_size(),
        "image data size mismatch"
    );

    // TODO check size of mip level
    let glfmt = GlFormatInfo::from_format(fmt);

    let mut prev_unpack_alignment = 0;
    gl.GetIntegerv(gl::UNPACK_ALIGNMENT, &mut prev_unpack_alignment);
    gl.PixelStorei(gl::UNPACK_ALIGNMENT, 1);

    match target {
        gl::TEXTURE_1D => {
            gl.TextureSubImage1D(
                img,
                mip_level,
                offset.0 as i32,
                size.0 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        }
        gl::TEXTURE_2D => {
            gl.TextureSubImage2D(
                img,
                mip_level,
                offset.0 as i32,
                offset.1 as i32,
                size.0 as i32,
                size.1 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        }
        gl::TEXTURE_3D => {
            gl.TextureSubImage3D(
                img,
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
        }
        _ => unimplemented!(),
    };

    gl.PixelStorei(gl::UNPACK_ALIGNMENT, prev_unpack_alignment);
}

//--------------------------------------------------------------------------------------------------

new_key_type! {
    pub struct ImageAliasKey;
}

/// OpenGL image.
///
/// It can be either a texture object or a renderbuffer object if sampling is not required.
#[derive(Debug)]
pub struct GlImage {
    pub(crate) raw: RawImage,
    pub(crate) should_destroy: bool,
    pub(crate) alias_info: Option<AliasInfo<ImageAliasKey>>,
}
