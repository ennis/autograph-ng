use crate::{
    api as gl,
    api::{types::*, Gl},
    image::GlImage,
};

/// Wrapper around OpenGL framebuffers.
#[derive(Debug)]
pub(crate) struct GlFramebuffer {
    pub(crate) obj: GLuint,
}

impl GlFramebuffer {
    /// Creates a new OpenGL framebuffer object (FBO).
    ///
    /// The specified _n_ color attachments are bound to GL_COLOR_ATTACHMENT0 to
    /// GL_COLOR_ATTACHMENT_n_. The _n_ first draw buffers of the FBO are enabled,
    /// and mapped to the color attachments.
    /// For texture attachments, the topmost layer is attached.
    ///
    /// Panics if the number of color attachments is greater than 8.
    ///
    pub(crate) fn new(
        gl: &Gl,
        color_attachments: &[&GlImage],
        depth_stencil_attachment: Option<&GlImage>,
    ) -> Result<GlFramebuffer, GLenum> {
        assert!(color_attachments.len() < 8);

        let mut obj = 0;
        unsafe {
            gl.CreateFramebuffers(1, &mut obj);
        }

        // color attachments
        for (index, &img) in color_attachments.iter().enumerate() {
            let index = index as u32;
            match img.raw.target {
                gl::RENDERBUFFER => unsafe {
                    gl.NamedFramebufferRenderbuffer(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        gl::RENDERBUFFER,
                        img.raw.obj,
                    );
                },
                _ => unsafe {
                    gl.NamedFramebufferTexture(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        img.raw.obj,
                        0, // TODO
                    );
                },
            }
        }

        // depth-stencil attachment
        if let Some(img) = depth_stencil_attachment {
            match img.raw.target {
                gl::RENDERBUFFER => unsafe {
                    gl.NamedFramebufferRenderbuffer(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        gl::RENDERBUFFER,
                        img.raw.obj,
                    );
                },
                _ => unsafe {
                    gl.NamedFramebufferTexture(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        img.raw.obj,
                        0, // TODO
                    );
                },
            }
        }

        // enable draw buffers
        unsafe {
            gl.NamedFramebufferDrawBuffers(
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
        let status = unsafe { gl.CheckNamedFramebufferStatus(obj, gl::DRAW_FRAMEBUFFER) };

        if status == gl::FRAMEBUFFER_COMPLETE {
            Ok(GlFramebuffer { obj: dbg!(obj) })
        } else {
            Err(status)
        }
    }

    /// Destroys this framebuffer object.
    pub(crate) fn destroy(self, gl: &Gl) {
        unsafe {
            gl.DeleteFramebuffers(1, &self.obj);
        }
    }
}
