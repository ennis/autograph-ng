use crate::renderer;
use crate::renderer::backend::gl::api as gl;
use crate::renderer::backend::gl::api::types::*;
use crate::renderer::backend::gl::{buffer::RawBuffer, image::RawImage};
use crate::renderer::backend::gl::{
    descriptor::{DescriptorSet, ShaderResourceBindings},
    resource::{Buffer, Image, Resources},
    state::StateCache,
    GraphicsPipeline, ImplementationParameters, OpenGlBackend, Swapchain,
};
use crate::renderer::{
    BufferTypeless, Command, CommandInner, IndexType, RendererBackend, ScissorRect, Viewport,
};
use glutin::GlWindow;

// resources
pub struct ExecuteContext<'a, 'rcx> {
    resources: &'a mut Resources,
    state_cache: &'a mut StateCache,
    window: &'a GlWindow,
    impl_params: &'a ImplementationParameters,
    current_pipeline: Option<&'rcx GraphicsPipeline>,
}

impl<'a, 'rcx> ExecuteContext<'a, 'rcx> {
    pub fn new(
        resources: &'a mut Resources,
        state_cache: &'a mut StateCache,
        window: &'a GlWindow,
        impl_params: &'a ImplementationParameters,
    ) -> ExecuteContext<'a, 'rcx> {
        ExecuteContext {
            resources,
            state_cache,
            window,
            impl_params,
            current_pipeline: None,
        }
    }

    pub fn cmd_clear_image_float(&mut self, image: &Image, color: &[f32; 4]) {
        if image.target == gl::RENDERBUFFER {
            // create temporary framebuffer
            let mut tmpfb = 0;
            unsafe {
                gl::CreateFramebuffers(1, &mut tmpfb);
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::COLOR_ATTACHMENT0,
                    gl::RENDERBUFFER,
                    image.obj,
                );
                gl::NamedFramebufferDrawBuffers(tmpfb, 1, (&[gl::COLOR_ATTACHMENT0]).as_ptr());
                gl::ClearNamedFramebufferfv(tmpfb, gl::COLOR, 0, color.as_ptr());
                gl::DeleteFramebuffers(1, &tmpfb);
            }
        } else {
            // TODO specify which level to clear in command
            unsafe {
                gl::ClearTexImage(
                    image.obj,
                    0,
                    gl::RGBA,
                    gl::FLOAT,
                    color.as_ptr() as *const _,
                );
            }
        }
    }

    pub fn cmd_clear_depth_stencil_image(
        &mut self,
        image: &Image,
        depth: f32,
        stencil: Option<u8>,
    ) {
        let obj = image.obj;
        if image.target == gl::RENDERBUFFER {
            // create temporary framebuffer
            let mut tmpfb = 0;
            unsafe {
                gl::CreateFramebuffers(1, &mut tmpfb);
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::DEPTH_ATTACHMENT,
                    gl::RENDERBUFFER,
                    obj,
                );
                if let Some(stencil) = stencil {
                    unimplemented!()
                } else {
                    gl::ClearNamedFramebufferfv(tmpfb, gl::DEPTH, 0, &depth);
                }
                gl::DeleteFramebuffers(1, &tmpfb);
            }
        } else {
            // TODO specify which level to clear in command
            unsafe {
                if let Some(stencil) = stencil {
                    unimplemented!()
                } else {
                    gl::ClearTexImage(
                        obj,
                        0,
                        gl::DEPTH_COMPONENT,
                        gl::FLOAT,
                        &depth as *const f32 as *const _,
                    );
                }
            }
        }
    }

    //pub fn cmd_set_attachments(&mut self, color_attachments: &[R::])

    pub fn cmd_set_descriptor_sets(
        &mut self,
        descriptor_sets: &[renderer::DescriptorSet<'rcx, OpenGlBackend>],
    ) {
        let pipeline = self.current_pipeline.unwrap();
        let descriptor_map = pipeline.descriptor_map();
        let mut sr = ShaderResourceBindings::new();

        for (i, &ds) in descriptor_sets.iter().enumerate() {
            ds.0.collect(i as u32, descriptor_map, &mut sr);
        }

        self.state_cache.set_uniform_buffers(
            &sr.uniform_buffers,
            &sr.uniform_buffer_offsets,
            &sr.uniform_buffer_sizes,
        );
        self.state_cache.set_shader_storage_buffers(
            &sr.shader_storage_buffers,
            &sr.shader_storage_buffer_offsets,
            &sr.shader_storage_buffer_sizes,
        );
    }

    pub fn cmd_present(&mut self, image: &Image, swapchain: &Swapchain) {
        // only handle default swapchain for now
        //assert_eq!(swapchain, 0, "invalid swapchain handle");
        // make a framebuffer and bind the image to it
        unsafe {
            let mut tmpfb = 0;
            gl::CreateFramebuffers(1, &mut tmpfb);
            // bind image to it
            if image.target == gl::RENDERBUFFER {
                gl::NamedFramebufferRenderbuffer(
                    tmpfb,
                    gl::COLOR_ATTACHMENT0,
                    gl::RENDERBUFFER,
                    image.obj,
                );
            } else {
                // TODO other levels / layers?
                gl::NamedFramebufferTexture(tmpfb, gl::COLOR_ATTACHMENT0, image.obj, 0);
            }
            // blit to default framebuffer
            //gl::BindFramebuffer(gl::READ_FRAMEBUFFER, tmpfb);
            let (w, h): (u32, u32) = self.window.get_inner_size().unwrap().into();

            gl::BlitNamedFramebuffer(
                tmpfb,
                0,
                0,        // srcX0
                0,        // srcY0
                w as i32, // srcX1,
                h as i32, // srcY1,
                0,        // dstX0,
                0,        // dstY0,
                w as i32, // dstX1
                h as i32, // dstY1
                gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT,
                gl::NEAREST,
            );

            // destroy temp framebuffer
            gl::DeleteFramebuffers(1, &tmpfb);
        }

        // swap buffers
        self.window.swap_buffers().expect("swap_buffers error")
    }

    fn cmd_set_graphics_pipeline(&mut self, pipeline: &'rcx GraphicsPipeline) {
        // switching pipelines
        self.current_pipeline = Some(pipeline);
        pipeline.bind(self.state_cache);
    }

    fn cmd_set_vertex_buffers(&mut self, buffers: &[BufferTypeless<'rcx, OpenGlBackend>]) {
        let pipeline = self
            .current_pipeline
            .expect("cmd_set_vertex_buffers called with no pipeline bound");
        let vertex_input_bindings = pipeline.vertex_input_bindings();

        let mut objs = smallvec::SmallVec::<[GLuint; 8]>::new();
        let mut offsets = smallvec::SmallVec::<[GLintptr; 8]>::new();
        let mut strides = smallvec::SmallVec::<[GLsizei; 8]>::new();

        for (i, &vb) in buffers.iter().enumerate() {
            objs.push(vb.0.obj);
            offsets.push(vb.0.offset as isize);
            strides.push(vertex_input_bindings[i].stride as i32);
        }

        self.state_cache
            .set_vertex_buffers(&objs, &offsets, &strides);
    }

    fn cmd_set_viewports(&mut self, viewports: &[Viewport]) {
        self.state_cache.set_viewports(viewports);
    }

    fn cmd_set_index_buffer(&mut self, index_buffer: &'rcx Buffer, offset: usize, ty: IndexType) {
        self.state_cache
            .set_index_buffer(index_buffer.obj, offset, ty);
    }

    pub fn execute_command(&mut self, command: &Command<'rcx, OpenGlBackend>) {
        match command.cmd {
            CommandInner::PipelineBarrier {} => {
                // no-op on GL
            }
            CommandInner::ClearImageFloat { image, color } => {
                self.cmd_clear_image_float(image.0, &color);
            }
            CommandInner::ClearDepthStencilImage {
                image,
                depth,
                stencil,
            } => {
                self.cmd_clear_depth_stencil_image(image.0, depth, stencil);
            }
            CommandInner::SetDescriptorSets {
                ref descriptor_sets,
            } => {
                self.cmd_set_descriptor_sets(descriptor_sets);
            }
            CommandInner::SetVertexBuffers { ref vertex_buffers } => {
                self.cmd_set_vertex_buffers(vertex_buffers);
            }
            CommandInner::SetIndexBuffer {
                index_buffer,
                offset,
                ty,
            } => {
                self.cmd_set_index_buffer(index_buffer.0, offset, ty);
            }
            CommandInner::DrawHeader { pipeline } => {
                self.cmd_set_graphics_pipeline(pipeline.0);
            }
            CommandInner::SetScissors { .. } => {}
            //CommandInner::SetAllScissors { scissor } => {}
            CommandInner::SetViewports { ref viewports } => {
                self.cmd_set_viewports(viewports);
            }
            //CommandInner::SetAllViewports { viewport } => {}
            CommandInner::SetFramebuffer { framebuffer } => {}
            CommandInner::Draw { .. } => unimplemented!(),
            CommandInner::Present { image, swapchain } => {
                self.cmd_present(image.0, swapchain.0);
            }
            _ => unimplemented!(),
        }
    }
}
