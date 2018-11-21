use crate::renderer::format::*;
use crate::renderer::image::*;

use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::backend::gl::format::*;

use std::cmp::*;

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
            Dimensions::Dim1d { width } => ExtentsAndType {
                target: gl::TEXTURE_1D,
                width,
                height: 1,
                depth: 1,
                array_layers: 1,
            },
            Dimensions::Dim1dArray {
                width,
                array_layers,
            } => ExtentsAndType {
                target: gl::TEXTURE_2D,
                width,
                height: 1,
                depth: 1,
                array_layers,
            },
            Dimensions::Dim2d { width, height } => ExtentsAndType {
                target: gl::TEXTURE_2D,
                width,
                height,
                depth: 1,
                array_layers: 1,
            },
            Dimensions::Dim2dArray {
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

/*
bitflags! {
    #[derive(Default)]
    pub struct TextureOptions: u8 {
        ///
        const SPARSE_STORAGE = 0b00000001;
    }
}
*/

//--------------------------------------------------------------------------------------------------


/// Wrapper for OpenGL textures and renderbuffers.
/// Copy + Clone to bypass a restriction of slotmap on stable rust.
#[derive(Copy,Clone,Debug)]
pub struct Image {
    pub obj: GLuint,
    pub target: GLenum,
}

impl Image {
    pub fn new_texture(
        format: Format,
        dimensions: &Dimensions,
        mipmaps: MipmapsCount,
        samples: u32,
    ) -> Image {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = GlFormatInfo::from_format(format);

        let mipcount = match mipmaps {
            MipmapsCount::Log2 => get_texture_mip_map_count(max(et.width, et.height)),
            MipmapsCount::Specific(count) => {
                // Multisampled textures can't have more than one mip level
                if samples > 1 {
                    assert_eq!(count, 1);
                }
                count
            }
            MipmapsCount::One => 1,
        };

        let mut obj = 0;
        unsafe {
            gl::CreateTextures(et.target, 1, &mut obj);

            /*if desc.options.contains(SPARSE_STORAGE) {
                gl::TextureParameteri(obj, gl::TEXTURE_SPARSE_ARB, gl::TRUE as i32);
            }*/

            match et.target {
                gl::TEXTURE_1D => {
                    gl::TextureStorage1D(obj, mipcount as i32, glfmt.internal_fmt, et.width as i32);
                }
                gl::TEXTURE_2D => {
                    if samples > 1 {
                        gl::TextureStorage2DMultisample(
                            obj,
                            samples as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                            true as u8,
                        );
                    } else {
                        gl::TextureStorage2D(
                            obj,
                            mipcount as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                        );
                    }
                }
                gl::TEXTURE_3D => {
                    gl::TextureStorage3D(
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

            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TextureParameteri(obj, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }

        Image {
            obj,
            target: et.target,
        }
    }

    pub fn new_renderbuffer(
        format: Format,
        dimensions: &Dimensions,
        samples: u32) -> Image
    {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = GlFormatInfo::from_format(format);

        let mut obj = 0;
        gl::CreateRenderbuffers(1, &mut obj);

        if samples > 1 {
            gl::RenderbufferStorageMultisample(
                obj,
                samples as i32,
                glfmt.internal_fmt,
                et.width as i32,
                et.height as i32,
            );
        } else {
            gl::RenderbufferStorage(
                obj,
                glfmt.internal_fmt,
                et.width as i32,
                et.height as i32,
            );
        }

        Image {
            obj,
            target: gl::RENDERBUFFER,
        }
    }

    pub fn is_renderbuffer(&self) -> bool {
        self.target == gl::RENDERBUFFER
    }
}

/// Texture upload
pub unsafe fn upload_image_region(
    img: &Image,
    fmt: Format,
    mip_level: i32,
    offset: (u32, u32, u32),
    size: (u32, u32, u32),
    data: &[u8],
) {
    if img.is_renderbuffer() {
        panic!("image does not support upload")
    }

    let fmtinfo = fmt.get_format_info();
    assert_eq!(
        data.len(),
        (size.0 * size.1 * size.2) as usize * fmtinfo.byte_size(),
        "image data size mismatch"
    );

    // TODO check size of mip level
    let glfmt = GlFormatInfo::from_format(fmt);

    let mut prev_unpack_alignment = 0;
    unsafe {
        gl::GetIntegerv(gl::UNPACK_ALIGNMENT, &mut prev_unpack_alignment);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
    };

    match img.target {
        gl::TEXTURE_1D => unsafe {
            gl::TextureSubImage1D(
                img.obj,
                mip_level,
                offset.0 as i32,
                size.0 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        },
        gl::TEXTURE_2D => unsafe {
            gl::TextureSubImage2D(
                img.obj,
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
        gl::TEXTURE_3D => unsafe {
            gl::TextureSubImage3D(
                img.obj,
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
        _ => unimplemented!(),
    };

    unsafe {
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, prev_unpack_alignment);
    }
}
