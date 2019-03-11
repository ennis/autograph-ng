//! Renderer for dear imgui (https://github.com/ocornut/imgui) using autograph-render as a backend.
#![feature(proc_macro_hygiene)]
use autograph_render::{
    buffer::{Buffer, StructuredBufferData, TypedConstantBufferView},
    command::{CommandBuffer, DrawIndexedParams},
    format::Format,
    glm,
    image::{
        ImageUsageFlags, MipmapsOption, RenderTarget2dView, SamplerDescription,
        TextureSampler2dView,
    },
    include_glsl,
    pipeline::{
        Arguments, ColorBlendState, DepthStencilState, GraphicsPipelineCreateInfo,
        InputAssemblyState, MultisampleState, RasterisationState, ReflectedShader, Scissor,
        ScissorRect, TypedArgumentBlock, TypedGraphicsPipeline, Viewport, ViewportState,
    },
    vertex::{IndexBufferView, VertexBufferView, VertexData},
    Arena, Backend,
};
use imgui::ImGui;
use std::{mem, slice};

/// ImGui vertex shader
static IMGUI_VERT: ReflectedShader = include_glsl!("imgui.vert");
/// ImGui fragment shader
static IMGUI_FRAG: ReflectedShader = include_glsl!("imgui.frag");

/// Vertices produced by dear imgui.
#[derive(Copy, Clone, Debug, VertexData)]
#[repr(C)]
struct ImDrawVert {
    pos: glm::Vec2,
    tex: glm::Vec2,
    color: glm::U8Vec4,
}

#[derive(Copy, Clone, Debug, StructuredBufferData)]
#[repr(C)]
struct ImUniforms {
    mat: glm::Mat4,
}

#[derive(Copy, Clone, Debug, Arguments)]
struct ImRenderTarget<'a, B: Backend> {
    #[argument(render_target)]
    target: RenderTarget2dView<'a, B>,
    #[argument(viewport)]
    viewport: Viewport,
}

#[derive(Clone, Debug, Arguments)]
struct ImArguments<'a, B: Backend> {
    #[argument(inherit)]
    rt: TypedArgumentBlock<'a, B, ImRenderTarget<'a, B>>,
    #[argument(descriptor)]
    uniforms: TypedConstantBufferView<'a, B, ImUniforms>,
    #[argument(descriptor)]
    tex: TextureSampler2dView<'a, B>,
    #[argument(vertex_buffer)]
    vertices: Buffer<'a, B, [ImDrawVert]>,
    #[argument(index_buffer)]
    indices: Buffer<'a, B, [u16]>,
    #[argument(scissor)]
    scissor: Scissor,
}

fn create_pipeline<'a, B: Backend>(
    arena: &'a Arena<B>,
) -> TypedGraphicsPipeline<'a, B, ImArguments<'a, B>> {
    let create_info = GraphicsPipelineCreateInfo {
        shader_stages: arena.create_vertex_fragment_shader_stages(IMGUI_VERT, IMGUI_FRAG),
        viewport_state: ViewportState::DYNAMIC_VIEWPORT_SCISSOR,
        rasterization_state: RasterisationState::default(),
        multisample_state: MultisampleState::default(),
        depth_stencil_state: DepthStencilState::default(),
        input_assembly_state: InputAssemblyState::default(),
        color_blend_state: ColorBlendState::ALPHA_BLENDING,
    };

    arena.create_graphics_pipeline(&create_info)
}

/// Renderer for dear imgui.
pub struct ImGuiRenderer<'a, B: Backend> {
    pipeline: TypedGraphicsPipeline<'a, B, ImArguments<'a, B>>,
    render_target: TypedArgumentBlock<'a, B, ImRenderTarget<'a, B>>,
    font_tex: TextureSampler2dView<'a, B>,
}

impl<'a, B: Backend> ImGuiRenderer<'a, B> {
    /// Creates a new renderer.
    ///
    /// The imgui frames are rendered into the image specified by `target`, within the specified
    /// `viewport`.
    /// `arena` is the arena that should be used to allocate resources that live as long as the
    /// renderer (graphics pipelines, font textures, etc.).
    pub fn new(
        arena: &'a Arena<B>,
        ui: &mut ImGui,
        target: RenderTarget2dView<'a, B>,
        viewport: Viewport,
    ) -> ImGuiRenderer<'a, B> {
        // sanity check
        assert_eq!(
            mem::size_of::<imgui::ImDrawVert>(),
            mem::size_of::<ImDrawVert>()
        );

        let pipeline = create_pipeline(arena);

        let font_tex = ui.prepare_texture(|handle| {
            let texture = arena
                .image_2d(Format::R8G8B8A8_SRGB, handle.width, handle.height)
                .with_data(handle.pixels);
            texture
        });

        let render_target = arena.create_typed_argument_block(ImRenderTarget { target, viewport });

        ImGuiRenderer {
            pipeline,
            font_tex: font_tex.sampled(SamplerDescription::NEAREST_MIPMAP_NEAREST),
            render_target,
        }
    }

    fn render_draw_list<'b>(
        &self,
        ui: &imgui::Ui,
        frame_arena: &'b Arena<'b, B>,
        cmdbuf: &mut CommandBuffer<'b, B>,
        sortkey: u64,
        draw_list: &imgui::DrawList,
    ) where
        'a: 'b,
    {
        let vertices = unsafe {
            slice::from_raw_parts(
                draw_list.vtx_buffer.as_ptr() as *const ImDrawVert,
                draw_list.vtx_buffer.len(),
            )
        };
        let vertices = frame_arena.upload_slice(vertices);
        let indices = frame_arena.upload_slice(draw_list.idx_buffer);
        let (width, height) = ui.imgui().display_size();
        let (scale_width, scale_height) = ui.imgui().display_framebuffer_scale();

        if width == 0.0 || height == 0.0 {
            return;
        }

        let mat = glm::transpose(&glm::mat4(
            2.0 / width as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / height as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            -1.0,
            -1.0,
            0.0,
            1.0,
        ));

        let mut idx_start = 0u32;

        for cmd in draw_list.cmd_buffer.iter() {
            let scissor = ScissorRect {
                x: (cmd.clip_rect.x * scale_width) as i32,
                y: (cmd.clip_rect.y * scale_height) as i32,
                width: ((cmd.clip_rect.z - cmd.clip_rect.x) * scale_width) as u32,
                height: ((cmd.clip_rect.w - cmd.clip_rect.y) * scale_height) as u32,
            };

            let args = frame_arena.create_typed_argument_block(ImArguments {
                rt: self.render_target,
                uniforms: frame_arena.upload(&ImUniforms { mat }).into(),
                vertices,
                tex: self.font_tex,
                indices,
                scissor: Scissor::Enabled(scissor),
            });

            cmdbuf.draw_indexed(
                sortkey,
                frame_arena,
                self.pipeline,
                args,
                DrawIndexedParams {
                    index_count: cmd.elem_count,
                    instance_count: 1,
                    first_index: idx_start,
                    vertex_offset: 0,
                    first_instance: 0,
                },
            );

            idx_start += cmd.elem_count;
        }
    }

    /// Renders the specified imgui frame into a command buffer.
    ///
    /// Arguments:
    /// - `cmdbuf`: the command buffer to push rendering commands into
    /// - `sortkey`: sorting key for the rendering commands. All commands pushed into the command
    ///     buffer share the same sorting key.
    /// - `frame_arena`: the arena that should be used to allocate the temporary resources
    ///  (vertex buffers, index buffers, etc.) necessary for the rendering commands.
    /// - `ui`: the imgui frame to render
    pub fn render<'b>(
        &mut self,
        cmdbuf: &mut CommandBuffer<'b, B>,
        sortkey: u64,
        frame_arena: &'b Arena<'b, B>,
        ui: imgui::Ui,
    ) where
        'a: 'b,
    {
        ui.render(move |ui, draw_data| -> Result<(), String> {
            for draw_list in &draw_data {
                self.render_draw_list(ui, frame_arena, cmdbuf, sortkey, &draw_list);
            }
            Ok(())
        })
        .unwrap();
    }
}
