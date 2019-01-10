#[macro_use]
extern crate log;

mod common;

use self::common::*;
use gfx2;
use gfx2::glm;
use gfx2::interface::DescriptorSetInterface;
use gfx2::interface::FragmentOutputDescription;
use gfx2::interface::PipelineInterface;
use gfx2::interface::PipelineInterfaceVisitor;
use gfx2::interface::VertexInputBufferDescription;
use gfx2::*;
use gfx2_backend_gl as gl_backend;
use gfx2_extension_runtime::{load_dev_dylib, load_module};
use std::env;

//--------------------------------------------------------------------------------------------------
type Backend = gl_backend::OpenGlBackend;
type Buffer<'a, T> = gfx2::Buffer<'a, Backend, T>;
type SampledImage<'a> = gfx2::SampledImage<'a, Backend>;
type Framebuffer<'a> = gfx2::Framebuffer<'a, Backend>;
type DescriptorSet<'a> = gfx2::DescriptorSet<'a, Backend>;
type DescriptorSetLayout<'a> = gfx2::DescriptorSetLayout<'a, Backend>;
type GraphicsPipeline<'a> = gfx2::GraphicsPipeline<'a, Backend>;

//--------------------------------------------------------------------------------------------------
#[derive(Copy, Clone)]
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

#[derive(BufferLayout, Copy, Clone)]
#[repr(C)]
pub struct Uniforms {
    pub transform: glm::Mat4x3,
    pub resolution: glm::Vec2,
}

#[derive(DescriptorSetInterface)]
#[interface(arguments = "<'a,Backend>")]
pub struct PerObject<'a> {
    #[descriptor(uniform_buffer)]
    pub uniforms: Buffer<'a, Uniforms>,
    #[descriptor(sampled_image)]
    pub image: SampledImage<'a>,
    #[descriptor(sampled_image)]
    pub dither: SampledImage<'a>,
}

pub struct Blit<'a> {
    pub framebuffer: Framebuffer<'a>,
    pub per_object: DescriptorSet<'a>,
    pub viewport: Viewport,
    pub vertex_buffer: Buffer<'a, [Vertex]>,
}

impl<'a> PipelineInterface<'a, Backend> for Blit<'a> {
    const VERTEX_INPUT_INTERFACE: &'static [VertexInputBufferDescription<'static>] = &[];
    const FRAGMENT_OUTPUT_INTERFACE: &'static [FragmentOutputDescription] = &[];
    const DESCRIPTOR_SET_INTERFACE: &'static [&'static [DescriptorSetLayoutBinding<'static>]] = &[];

    fn do_visit(&self, visitor: &mut PipelineInterfaceVisitor<'a, Backend>) {
        visitor.visit_dynamic_viewports(&[self.viewport]);
        visitor.visit_vertex_buffers(&[self.vertex_buffer.into()]);
        visitor.visit_framebuffer(self.framebuffer);
        visitor.visit_descriptor_sets(&[self.per_object]);
    }
}

//--------------------------------------------------------------------------------------------------
// SHADERS & PIPELINES
struct PipelineAndLayout<'a> {
    blit_pipeline: GraphicsPipeline<'a>,
    descriptor_set_layout: DescriptorSetLayout<'a>,
}

fn create_pipelines<'a>(arena: &'a Arena<Backend>, vs: &[u8], fs: &[u8]) -> PipelineAndLayout<'a> {
    let descriptor_set_layout = arena.create_descriptor_set_layout(PerObject::INTERFACE);

    let gci = GraphicsPipelineCreateInfo {
        shader_stages: &GraphicsShaderStages {
            vertex: arena.create_shader_module(vs, ShaderStageFlags::VERTEX),
            geometry: None,
            fragment: Some(arena.create_shader_module(fs, ShaderStageFlags::FRAGMENT)),
            tess_eval: None,
            tess_control: None,
        },
        vertex_input_state: &VertexInputState {
            bindings: &[VertexInputBindingDescription {
                binding: 0,
                stride: 16,
                input_rate: VertexInputRate::Vertex,
            }],
            attributes: &[
                VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: 0,
                },
                VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: Format::R32G32_SFLOAT,
                    offset: 8,
                },
            ],
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
        dynamic_state: DynamicStateFlags::VIEWPORT,
        pipeline_layout: &PipelineLayout {
            descriptor_set_layouts: &[descriptor_set_layout],
        },
        attachment_layout: &AttachmentLayout {
            input_attachments: &[],
            depth_attachment: None,
            color_attachments: &[AttachmentDescription {
                format: Format::R8G8B8A8_SRGB,
                samples: 1,
            }],
        },
    };

    PipelineAndLayout {
        blit_pipeline: arena.create_graphics_pipeline(&gci),
        descriptor_set_layout,
    }
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
        let pipeline = create_pipelines(&arena_0, shader_mod.BLIT_VERT, shader_mod.BLIT_FRAG);

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

                let per_object = arena_2.create_descriptor_set(
                    pipeline.descriptor_set_layout,
                    PerObject {
                        uniforms,
                        image,
                        dither,
                    },
                );

                let mut cmdbuf = r.create_command_buffer();
                cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);

                cmdbuf.draw(
                    0x0,
                    pipeline.blit_pipeline,
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
