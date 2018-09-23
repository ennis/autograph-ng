extern crate ash;
extern crate gfx2;

use std::env;

use gfx2::frame::*;
use gfx2::import::import_graph;
use gfx2::resource::*;
use gfx2::texture::get_texture_mip_map_count;
use gfx2::vk;
use gfx2::window::*;

//--------------------------------------------------------------------------------------------------
fn downsample(frame: &mut Frame, input: &ImageRef, aux: &ImageRef) -> ImageRef {
    let (w, h, d) = frame.get_image_dimensions(input);
    let count = get_texture_mip_map_count(w, h);

    let mut r_last = None;
    let mut cur_w = w;
    let mut cur_h = h;
    for i in 0..count {
        let (t, r_target) = frame.create_graphics_task("downsample", |t| {
            t.sample_image(r_last.as_ref().unwrap_or(input));
            t.create_attachment(
                AttachmentIndex::Color(0),
                (cur_w, cur_h),
                vk::Format::R16g16b16a16Sfloat,
            )
        });
        r_last = Some(r_target);
        cur_w /= 2;
        cur_h /= 2;
    }

    r_last.unwrap()
}

//--------------------------------------------------------------------------------------------------
fn test_frame_0<'ctx>(frame: &mut Frame<'ctx>, persistent: &'ctx mut Image) {
    let (t01, r01) = frame.create_task_on_queue("T01", TaskType::Graphics, 0, |t| {
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t02, r02) = frame.create_task_on_queue("T02", TaskType::Graphics, 0, |t| {
        t.sample_image(&r01);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t11, r11) = frame.create_task_on_queue("T11", TaskType::Graphics, 1, |t| {
        t.sample_image(&r02);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t12, r12) = frame.create_task_on_queue("T12", TaskType::Compute, 1, |t| {
        t.sample_image(&r11);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t13, mut r13) = frame.create_task_on_queue("T13", TaskType::Compute, 1, |t| {
        t.sample_image(&r02);
        t.sample_image(&r11);
        t.sample_image(&r12);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t03, mut r03) = frame.create_task_on_queue("T03", TaskType::Graphics, 0, |t| {
        t.sample_image(&r13);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t04, mut r04) = frame.create_task_on_queue("T04", TaskType::Graphics, 0, |t| {
        t.sample_image(&r03);
        t.sample_image(&r12);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let t05 = frame.create_task_on_queue("T05", TaskType::Graphics, 0, |t| {
        t.sample_image(&r04);
    });

    let (t21, mut r21) = frame.create_task_on_queue("T21", TaskType::Present, 2, |t| {
        t.sample_image(&r12);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let (t14, mut r14) = frame.create_task_on_queue("T14", TaskType::Compute, 1, |t| {
        t.sample_image(&r12);
        t.sample_image(&r04);
        t.sample_image(&r21);
        t.create_attachment(
            AttachmentIndex::Color(0),
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
        )
    });

    let mut r_output = frame.import_image(persistent);
    let t22 = frame.create_task_on_queue("T22", TaskType::Present, 2, |t| {
        t.sample_image(&r12);
        t.sample_image(&r14);
        t.attachment(AttachmentIndex::Color(0), &mut r_output);
    });
}

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();
    let ctx = &mut app.context;
    let window = &mut app.window;

    // create a persistent image.
    let mut persistent_img = ctx.create_image_2d((1024, 1024), vk::Format::R8g8b8a8Srgb);

    let mut first = true;
    loop {
        let mut should_close = false;
        app.events_loop.poll_events(|event| {
            // event handling
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    should_close = true;
                }
                // if resize, then delete persistent resources and re-create
                _ => (),
            }

            if first {
                let mut frame = ctx.new_frame();
                test_frame_0(&mut frame, &mut persistent_img);
                frame.submit();
                first = false;
            }

            // ---- create frame (lock context)
            // let mut frame = ctx.new_frame();
            // ---- get the image associated with the presentation. (A)
            // let presentation_image = frame.presentation_image(app.presentation);
            // ---- clear presentation image (B)
            // frame.clear(presentation_image);
            // ---- submit frame (C)
            // frame.submit();

            // Dependency types:
            // - (R) texture (sampled image) (VK_ACCESS_SHADER_READ_BIT)
            // - (R/RW) storage image (VK_ACCESS_SHADER_READ_BIT | VK_ACCESS_SHADER_WRITE_BIT)
            // - (R) attachment input (VK_ACCESS_INPUT_ATTACHMENT_READ_BIT )
            // - (RW) depth attachment (VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT | VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT)
            // - (RW) stencil attachment (VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_READ_BIT | VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT)
            // - (RW) color attachment (VK_ACCESS_COLOR_ATTACHMENT_READ_BIT)
            // - (RW) buffer
            // -> Determines accessMask for barriers
            //
            // Dependency read barriers stage mask (for deps with R):
            // - by default: TOP_OF_PIPE
            // - ultimately: automagically detect from shader
            //
            // Dependency write barriers (for deps with W):
            // - by default: BOTTOM_OF_PIPE
            // - ultimately: automagically detect from shader
            //
            // Resource creation:
            // - in a node?
            // - outside, by the system?
            // - by specialized nodes: cannot both use and create a resource in the same node.

            // things that we need to clear the buffer:
            // (A) acquire next image in swapchain
            // (B) allocate command buffer
            // (B) transition image to render target
            // (B) create renderpass (from cache?)
            //      (B)

            // synchronize access with the persistent texture in the frame.
            //let mut persistent_tex = frame.sync(persistent_tex);
            // now persistent_tex can be used as a transient.
            // upload shared uniforms for this frame.
            // submit the frame to the command queue
            //frame.finish();
            //window.swap_buffers();
        });
        if should_close {
            break;
        }
    }

    // delete persistent textures.
    // context.release(persistent_tex);
}
