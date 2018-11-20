extern crate gfx2;

use std::env;

use gfx2::app::*;
use gfx2::renderer::*;

//--------------------------------------------------------------------------------------------------

/*
define_sort_key! {

    sequence:3 {
        MAIN => user_defined:25, pass_immediate:4,
        UI => user_defined,

        PRESENT => user_defined:25, pass_immediate:4
    }

    [sequence:3, layer:8, depth:16, pass_immediate:4],
    [opaque:3 = 3, layer:8, depth:16, pass_immediate:4],
    [shadow:3 = 1, view: 6, layer:8, depth:16, pass_immediate:4]

    sequence,objgroup,comp-pass(pre,draw,post),effect,effect-pass(pre,draw,post)
}

sequence_id!{ opaque, layer=group_id, depth=d, pass_immediate=0 }*/

pub struct RenderKey(u64);

impl RenderKey {}

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();

    let mut first = true;
    let mut should_close = false;

    while !should_close {
        should_close = app.poll_events(|event| {});

        let r = app.renderer();
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = r.swapchain_dimensions(default_swapchain);


        /*// create descriptor set (manually)
        let gbuffers_layout = r.create_descriptor_set_layout(&[
            LayoutBinding { stage_flags: SHADER_STAGE_ALL_GRAPHICS, descriptor_type: DescriptorType::SampledImage, count: 8 },  // 8 descriptors for G-buffers
            LayoutBinding { stage_flags: SHADER_STAGE_ALL_GRAPHICS, descriptor_type: DescriptorType::UniformBuffer, count: 1 }, // per-frame data
        ]);

        let per_object_layout = r.create_descriptor_set_layout(&[
            LayoutBinding { stage_flags: SHADER_STAGE_ALL_GRAPHICS, descriptor_type: DescriptorType::UniformBuffer, count: 1 }, // per-object data
        ]);

        // load pipeline
        let pipeline = r.create_pipeline(combined_shader_source, &[gbuffers_layout, per_object_layout])*/


        // register resources for the frame
        // FP16 color buffer
        let color_buffer = r.create_image(
            Format::R16G16B16A16_SFLOAT,
            (w, h).into(),
            MipmapsCount::One,
            1,
            ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLE,
            None,
        );

        // present color buffer
        let mut cmdbuf = r.create_command_buffer();
        cmdbuf.present(0xC000_0000, color_buffer, default_swapchain);

        r.submit_command_buffer(cmdbuf);
        r.end_frame();
    }

    // drop(persistent_img);
}
