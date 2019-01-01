use crate::{api as gl, api::types::*, OpenGlBackend as R};
use gfx2;

#[derive(Debug)]
pub struct Framebuffer {
    pub obj: GLuint,
}

impl Framebuffer {
    pub fn new(
        color_attachments: &[gfx2::Image<R>],
        depth_stencil_attachment: Option<gfx2::Image<R>>,
    ) -> Result<Framebuffer, GLenum> {
        let mut obj = 0;
        unsafe {
            gl::CreateFramebuffers(1, &mut obj);
        }

        // color attachments
        for (index, img) in color_attachments.iter().enumerate() {
            let index = index as u32;
            match img.0.target {
                gl::RENDERBUFFER => unsafe {
                    gl::NamedFramebufferRenderbuffer(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        gl::RENDERBUFFER,
                        img.0.obj,
                    );
                },
                _ => unsafe {
                    gl::NamedFramebufferTexture(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        img.0.obj,
                        0, // TODO
                    );
                },
            }
        }

        // depth-stencil attachment
        if let Some(img) = depth_stencil_attachment {
            match img.0.target {
                gl::RENDERBUFFER => unsafe {
                    gl::NamedFramebufferRenderbuffer(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        gl::RENDERBUFFER,
                        img.0.obj,
                    );
                },
                _ => unsafe {
                    gl::NamedFramebufferTexture(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        img.0.obj,
                        0, // TODO
                    );
                },
            }
        }

        // enable draw buffers
        unsafe {
            gl::NamedFramebufferDrawBuffers(
                obj,
                color_attachments.len() as i32,
                [
                    gl::COLOR_ATTACHMENT0,
                    gl::COLOR_ATTACHMENT0 + 1,
                    gl::COLOR_ATTACHMENT0 + 2,
                    gl::COLOR_ATTACHMENT0 + 3,
                    gl::COLOR_ATTACHMENT0 + 4,
                    gl::COLOR_ATTACHMENT0 + 5,
                    gl::COLOR_ATTACHMENT0 + 6,
                    gl::COLOR_ATTACHMENT0 + 7,
                ]
                .as_ptr(),
            )
        }

        // check framebuffer completeness
        let status = unsafe { gl::CheckNamedFramebufferStatus(obj, gl::DRAW_FRAMEBUFFER) };

        if status == gl::FRAMEBUFFER_COMPLETE {
            Ok(Framebuffer { obj })
        } else {
            Err(status)
        }
    }

    pub fn destroy(self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.obj);
        }
    }
}
