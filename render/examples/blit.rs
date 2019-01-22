#![feature(const_type_id)]

#[macro_use]
extern crate log;

pub mod common;

use self::common::*;
use autograph_plugin::{load_dev_dylib, load_module};
use autograph_render::buffer::Buffer;
use autograph_render::buffer::StructuredBufferData;
use autograph_render::command::DrawParams;
use autograph_render::descriptor::DescriptorSet;
use autograph_render::descriptor::DescriptorSetInterface;
use autograph_render::format::Format;
use autograph_render::framebuffer::Framebuffer;
use autograph_render::glm;
use autograph_render::image::ImageUsageFlags;
use autograph_render::image::MipmapsCount;
use autograph_render::image::SampledImage;
use autograph_render::image::SamplerDescription;
use autograph_render::pipeline::ColorBlendAttachmentState;
use autograph_render::pipeline::ColorBlendAttachments;
use autograph_render::pipeline::ColorBlendState;
use autograph_render::pipeline::DepthStencilState;
use autograph_render::pipeline::GraphicsPipeline;
use autograph_render::pipeline::GraphicsPipelineCreateInfo;
use autograph_render::pipeline::GraphicsShaderStages;
use autograph_render::pipeline::InputAssemblyState;
use autograph_render::pipeline::MultisampleState;
use autograph_render::pipeline::PipelineInterface;
use autograph_render::pipeline::PipelineLayout;
use autograph_render::pipeline::PrimitiveTopology;
use autograph_render::pipeline::RasterisationState;
use autograph_render::pipeline::Scissors;
use autograph_render::pipeline::ShaderStageFlags;
use autograph_render::pipeline::Viewport;
use autograph_render::pipeline::ViewportState;
use autograph_render::pipeline::Viewports;
use autograph_render::vertex::VertexData;
use autograph_render::AliasScope;
use autograph_render::Arena;
use std::env;

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone, VertexData)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub tex: [f32; 2],
}

impl Vertex {
    pub fn new(pos: [f32; 2], tex: [f32; 2]) -> Vertex {
        Vertex { pos, tex }
    }
}

//--------------------------------------------------------------------------------------------------
// SHADERS & PIPELINES
#[derive(StructuredBufferData, Copy, Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat4x3,
    pub resolution: glm::Vec2,
}

#[derive(DescriptorSetInterface)]
pub struct PerObject<'a> {
    #[descriptor(uniform_buffer)]
    pub uniforms: Buffer<'a, Uniforms>,
    #[descriptor(sampled_image)]
    pub image: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub dither: SampledImage<'a>,
}

#[derive(PipelineInterface)]
pub struct Blit<'a> {
    #[pipeline(framebuffer)]
    pub framebuffer: Framebuffer<'a>,
    #[pipeline(descriptor_set)]
    pub per_object: DescriptorSet<'a, PerObject<'a>>,
    #[pipeline(viewport)]
    pub viewport: Viewport,
    #[pipeline(vertex_buffer)]
    pub vertex_buffer: Buffer<'a, [Vertex]>,
}

fn create_pipelines<'a>(arena: &'a Arena, vs: &[u8], fs: &[u8]) -> GraphicsPipeline<'a, Blit<'a>> {
    let gci = GraphicsPipelineCreateInfo {
        shader_stages: &GraphicsShaderStages {
            vertex: arena.create_shader_module(vs, ShaderStageFlags::VERTEX),
            geometry: None,
            fragment: Some(arena.create_shader_module(fs, ShaderStageFlags::FRAGMENT)),
            tess_eval: None,
            tess_control: None,
        },
        viewport_state: &ViewportState {
            viewports: Viewports::Dynamic,
            scissors: Scissors::Dynamic,
        },
        rasterization_state: &RasterisationState::DEFAULT,
        multisample_state: &MultisampleState::default(),
        depth_stencil_state: &DepthStencilState::default(),
        input_assembly_state: &InputAssemblyState {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart_enable: false,
        },
        color_blend_state: &ColorBlendState {
            attachments: ColorBlendAttachments::All(&ColorBlendAttachmentState::DISABLED),
            blend_constants: [0.0.into(); 4],
            logic_op: None,
        },
    };

    arena.create_graphics_pipeline(&gci, &PipelineLayout::default())
}

//--------------------------------------------------------------------------------------------------
fn main() {
    //println!("OUT_DIR={}", env!("OUT_DIR"));
    println!("CARGO_MANIFEST_DIR={}", env!("CARGO_MANIFEST_DIR"));
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
    //env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let app = App::new();
    let r = app.renderer();

    // graphics pipelines
    'outer: loop {
        let arena_0 = r.create_arena();
        // reload shader crate
        let shader_lib = load_dev_dylib!(common_shaders).unwrap();
        let shader_mod = load_module!(&shader_lib, common_shaders::hot).unwrap();
        // reload pipelines
        let blit_pipeline = create_pipelines(&arena_0, shader_mod.BLIT_VERT, shader_mod.BLIT_FRAG);

        let image = arena_0
            .create_image(
                AliasScope::no_alias(),
                Format::R16G16B16A16_SFLOAT,
                (512, 512).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::SAMPLED,
            )
            .into_sampled(SamplerDescription::NEAREST_MIPMAP_NEAREST);

        let dither = common::load_image_2d(&arena_0, "tests/data/img/HDR_RGB_0.png")
            .unwrap()
            .into_sampled(SamplerDescription::WRAP_NEAREST_MIPMAP_NEAREST);

        let (left, top, right, bottom) = (-1.0, 1.0, 1.0, -1.0);

        let vertex_buffer = arena_0.upload_slice(&[
            Vertex::new([left, top], [0.0f32, 1.0f32]),
            Vertex::new([right, top], [1.0f32, 1.0f32]),
            Vertex::new([left, bottom], [0.0f32, 0.0f32]),
            Vertex::new([left, bottom], [0.0f32, 0.0f32]),
            Vertex::new([right, top], [1.0f32, 1.0f32]),
            Vertex::new([right, bottom], [1.0f32, 0.0f32]),
        ]);

        'swapchain: loop {
            let default_swapchain = r.default_swapchain().unwrap();
            let (w, h) = default_swapchain.size();
            let arena_1 = r.create_arena();

            let color_buffer = arena_1.create_image(
                AliasScope::no_alias(),
                Format::R16G16B16A16_SFLOAT,
                (w, h).into(),
                MipmapsCount::One,
                1,
                ImageUsageFlags::COLOR_ATTACHMENT,
            );

            let framebuffer = arena_1.create_framebuffer(&[color_buffer], None);

            'events: loop {
                let mut reload_shaders = false;

                let should_close = app.poll_events(|event| match event {
                    Event::WindowEvent {
                        event:
                            WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        virtual_keycode: Some(vkey),
                                        modifiers: mods,
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => {
                        info!("key={:?} mod={:?}", vkey, mods);
                        match vkey {
                            VirtualKeyCode::F5 => {
                                reload_shaders = true;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                });

                if shader_lib.should_reload() {
                    reload_shaders = true;
                }

                let arena_2 = r.create_arena();
                let uniforms = arena_2.upload(&Uniforms {
                    transform: glm::diagonal4x3(&glm::vec3(1.0, 1.0, 1.0)),
                    resolution: glm::vec2(w as f32, h as f32),
                });

                let per_object = PerObject {
                    uniforms,
                    image,
                    dither,
                }
                .into_descriptor_set(&arena_2);

                let mut cmdbuf = r.create_command_buffer();
                cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

                cmdbuf.draw(
                    0x0,
                    &arena_2,
                    blit_pipeline,
                    &Blit {
                        framebuffer,
                        per_object,
                        viewport: (w, h).into(),
                        vertex_buffer,
                    },
                    DrawParams {
                        instance_count: 1,
                        first_instance: 0,
                        vertex_count: 6,
                        first_vertex: 0,
                    },
                );

                cmdbuf.present(0x0, color_buffer, default_swapchain);
                r.submit_frame(vec![cmdbuf]);

                if should_close {
                    break 'outer;
                }

                if reload_shaders {
                    break 'swapchain;
                }

                let (new_w, new_h) = default_swapchain.size();
                // don't resize if new size is null in one dimension, as it will
                // cause create_framebuffer to fail.
                if (new_w, new_h) != (w, h) && new_w != 0 && new_h != 0 {
                    break 'events;
                }
            }
        }
    }
}
