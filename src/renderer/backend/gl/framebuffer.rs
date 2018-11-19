use super::context::Context;
use super::format::Format;
use super::texture::{Texture2D, TextureAny};
use gl;
use gl::types::*;
use glutin::GlWindow;
use std::cmp::max;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct RenderbufferObject {
    context: Context,
    format: Format,
    size: (u32, u32),
    obj: GLuint,
}

impl RenderbufferObject {
    pub fn new(
        _gctx: &Context,
        _width: u32,
        _height: u32,
        _format: Format,
        _num_samples: u32,
    ) -> RenderbufferObject {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub(super) enum OwnedFramebufferAttachment {
    Renderbuffer(Arc<RenderbufferObject>),
    Texture(TextureAny),
    TextureLayer(TextureAny, u32),
    Default,
    Empty,
}

#[derive(Debug)]
pub struct FramebufferObject {
    pub(super) gctx: Context,
    pub(super) size: (u32, u32),
    pub(super) obj: GLuint,
    pub(super) attachments: Vec<OwnedFramebufferAttachment>,
    pub(super) depth_attachment: OwnedFramebufferAttachment,
}

impl FramebufferObject {
    pub fn from_gl_window(gctx: &Context, window: &GlWindow) -> FramebufferObject {
        let pixel_size = window.get_inner_size().unwrap();
        FramebufferObject {
            gctx: gctx.clone(),
            size: (pixel_size.width as u32, pixel_size.height as u32),
            attachments: Vec::new(),
            depth_attachment: OwnedFramebufferAttachment::Default,
            obj: 0,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn gl_object(&self) -> GLuint {
        self.obj
    }
}

impl Drop for FramebufferObject {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &mut self.obj);
        }
    }
}

/// An OpenGL framebuffer object.
/// The framebuffer maintains a strong ref to attached textures and renderbuffers so it may outlive them.
#[derive(Clone, Debug, Deref)]
pub struct Framebuffer(Arc<FramebufferObject>);

impl Framebuffer {
    /// Returns the default framebuffer object associated to the given GlWindow
    pub fn from_gl_window(gctx: &Context, window: &GlWindow) -> Framebuffer {
        Framebuffer(Arc::new(FramebufferObject::from_gl_window(gctx, window)))
    }
}

#[derive(Clone, Debug)]
pub enum FramebufferAttachment<'a> {
    Renderbuffer(&'a Arc<RenderbufferObject>),
    Texture(&'a TextureAny),
    TextureLayer(&'a TextureAny, u32),
    Default,
    Empty,
}

impl<'a> FramebufferAttachment<'a> {
    fn to_owned(self) -> OwnedFramebufferAttachment {
        match self {
            FramebufferAttachment::Renderbuffer(renderbuffer) => {
                OwnedFramebufferAttachment::Renderbuffer(renderbuffer.clone())
            }
            FramebufferAttachment::Texture(texture) => {
                OwnedFramebufferAttachment::Texture(texture.clone())
            }
            FramebufferAttachment::TextureLayer(texture, layer) => {
                OwnedFramebufferAttachment::TextureLayer(texture.clone(), layer)
            }
            FramebufferAttachment::Default => OwnedFramebufferAttachment::Default,
            FramebufferAttachment::Empty => OwnedFramebufferAttachment::Empty,
        }
    }
}

/// Trait implemented by things that can be bound as a framebuffer attachement
/// (i.e. render targets)
pub trait ToFramebufferAttachment<'a> {
    fn to_framebuffer_attachement(self) -> FramebufferAttachment<'a>;
}

impl<'a> ToFramebufferAttachment<'a> for &'a Arc<RenderbufferObject> {
    fn to_framebuffer_attachement(self) -> FramebufferAttachment<'a> {
        FramebufferAttachment::Renderbuffer(self)
    }
}

impl<'a> ToFramebufferAttachment<'a> for &'a Texture2D {
    fn to_framebuffer_attachement(self) -> FramebufferAttachment<'a> {
        FramebufferAttachment::Texture(self)
    }
}

impl<'a> ToFramebufferAttachment<'a> for FramebufferAttachment<'a> {
    fn to_framebuffer_attachement(self) -> FramebufferAttachment<'a> {
        self
    }
}

pub struct FramebufferBuilder {
    gctx: Context,
    size: (u32, u32),
    attachments: Vec<OwnedFramebufferAttachment>,
    depth_attachment: OwnedFramebufferAttachment,
}

/// Errors that might happen when attaching renderbuffers or textures, or creating the framebuffer
#[derive(Debug, Fail)]
pub enum FramebufferError {
    #[fail(display = "attachment size mismatch")]
    AttachmentSizeMismatch,
    #[fail(display = "framebuffer validation failed")]
    ValidationFailed,
}

impl FramebufferBuilder {
    pub fn new(gctx: &Context) -> Self {
        FramebufferBuilder {
            gctx: gctx.clone(),
            size: (0, 0),
            attachments: Vec::new(),
            depth_attachment: OwnedFramebufferAttachment::Empty,
        }
    }

    fn check_or_update_size(
        &mut self,
        new: &OwnedFramebufferAttachment,
    ) -> Result<(), FramebufferError> {
        let size = match *new {
            OwnedFramebufferAttachment::Renderbuffer(ref renderbuffer) => Some(renderbuffer.size),
            OwnedFramebufferAttachment::Texture(ref texture) => {
                Some((texture.width(), texture.height()))
            }
            OwnedFramebufferAttachment::TextureLayer(ref texture, _) => unimplemented!(),
            OwnedFramebufferAttachment::Default => None,
            OwnedFramebufferAttachment::Empty => None,
        };

        if let Some(size) = size {
            if self.size == (0, 0) {
                self.size = size;
                Ok(())
            } else {
                if self.size == size {
                    Ok(())
                } else {
                    Err(FramebufferError::AttachmentSizeMismatch)
                }
            }
        } else {
            Ok(())
        }
    }

    /// ```rust
    /// let tex = gfx::Texture2D::new(...);
    /// fb.attach(&tex);
    /// fb.attach(FramebufferAttachement::TextureLayer(&tex,0));
    /// ```
    pub fn attach<'a, A: ToFramebufferAttachment<'a>>(
        &mut self,
        slot: u32,
        attachment: A,
    ) -> Result<(), FramebufferError> {
        let len = self.attachments.len();
        self.attachments.resize(
            max(slot as usize + 1, len),
            OwnedFramebufferAttachment::Empty,
        );
        let new = attachment.to_framebuffer_attachement().to_owned();
        self.check_or_update_size(&new)?;
        self.attachments.insert(slot as usize, new);
        Ok(())
    }

    pub fn attach_depth<'a, A: ToFramebufferAttachment<'a>>(
        &mut self,
        attachment: A,
    ) -> Result<(), FramebufferError> {
        let new = attachment.to_framebuffer_attachement().to_owned();
        self.check_or_update_size(&new)?;
        self.depth_attachment = new;
        Ok(())
    }

    pub fn build(self) -> Framebuffer {
        assert!(self.attachments.len() < 8);
        let mut obj = 0;
        unsafe {
            gl::CreateFramebuffers(1, &mut obj);
        }

        for (index, attachment) in self.attachments.iter().enumerate() {
            match attachment {
                &OwnedFramebufferAttachment::Texture(ref tex) => unsafe {
                    gl::NamedFramebufferTexture(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index as u32,
                        tex.gl_object(),
                        0,
                    );
                },
                &OwnedFramebufferAttachment::Renderbuffer(ref renderbuffer) => unsafe {
                    gl::NamedFramebufferRenderbuffer(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index as u32,
                        gl::RENDERBUFFER,
                        renderbuffer.obj,
                    );
                },
                &OwnedFramebufferAttachment::Empty => (),
                _ => unimplemented!("Framebuffer attachment not implemented"),
            }
        }

        unsafe {
            gl::NamedFramebufferDrawBuffers(
                obj,
                8,
                [
                    gl::COLOR_ATTACHMENT0,
                    gl::COLOR_ATTACHMENT0 + 1,
                    gl::COLOR_ATTACHMENT0 + 2,
                    gl::COLOR_ATTACHMENT0 + 3,
                    gl::COLOR_ATTACHMENT0 + 4,
                    gl::COLOR_ATTACHMENT0 + 5,
                    gl::COLOR_ATTACHMENT0 + 6,
                    gl::COLOR_ATTACHMENT0 + 7,
                ].as_ptr(),
            );
        }

        Framebuffer(Arc::new(FramebufferObject {
            obj,
            attachments: self.attachments,
            gctx: self.gctx,
            depth_attachment: self.depth_attachment,
            size: self.size,
        }))
    }
}
