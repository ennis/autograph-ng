extern crate ash;
extern crate gfx2;

use std::env;

use gfx2::import::import_graph;
use gfx2::vk;
use gfx2::*;

/*//--------------------------------------------------------------------------------------------------
fn downsample(frame: &mut Frame, input: &ImageRef, aux: &ImageRef) -> ImageRef {
    let (w, h, d) = frame.get_image_dimensions(input.id);
    //let count = get_texture_mip_map_count(w, h);

    let mut r_last = None;
    let mut cur_w = w;
    let mut cur_h = h;
    for i in 0..1 {
        let (t, r_target) = frame.create_graphics_task("downsample", |t| {
            t.sample_image(r_last.as_ref().unwrap_or(input));
            let (r, att) = t.create_attachment(
                (cur_w, cur_h),
                vk::Format::R16g16b16a16Sfloat,
                &AttachmentLoadStore::write_only(),
            );
            t.set_color_attachments(&[att]);
            r
        });
        r_last = Some(r_target);
        cur_w /= 2;
        cur_h /= 2;
    }

    r_last.unwrap()
}*/

//--------------------------------------------------------------------------------------------------
// somewhat closer to real-life
fn test_frame_deferred_shading<'ctx>(frame: &mut Frame<'ctx>, persistent: &'ctx mut Image) {
    let width = 1280;
    let height = 720;
    let dimensions = (width, height);

    let shadowmap_width = 1024;
    let shadowmap_height = 1024;
    let shadowmap_dimensions = (shadowmap_width, shadowmap_height);

    // init G-buffers and render
    //#[derive(AttachmentGroup)]
    struct Gbuffers {
        normals: AttachmentRef,
        diffuse_specular: AttachmentRef,
        emission: AttachmentRef,
        position: AttachmentRef,
        tangents: AttachmentRef,
        velocity: AttachmentRef,
        depth: AttachmentRef,
    }

    //
    let renderpass = frame.new_renderpass();

    let (_, gbuffers) = frame.graphics_subpass("gbuffers", renderpass, |t| {
        let normals = t.create_attachment(
            "normals",
            dimensions,
            vk::Format::R16g16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let diffuse_specular = t.create_attachment(
            "diffuse_specular",
            dimensions,
            vk::Format::R16g16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let position = t.create_attachment(
            "position",
            dimensions,
            vk::Format::R16g16b16a16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let emission = t.create_attachment(
            "emission",
            dimensions,
            vk::Format::R16g16b16a16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let tangents = t.create_attachment(
            "tangents",
            dimensions,
            vk::Format::R16g16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let velocity = t.create_attachment(
            "velocity",
            dimensions,
            vk::Format::R16g16Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );
        let depth = t.create_attachment(
            "depth",
            dimensions,
            vk::Format::D32Sfloat,
            vk::SAMPLE_COUNT_4_BIT,
            vk::AttachmentLoadOp::DontCare,
        );

        t.set_color_attachments(&[
            &normals,
            &diffuse_specular,
            &position,
            &emission,
            &tangents,
            &velocity,
        ]);
        t.set_depth_attachment(&depth);

        Gbuffers {
            normals,
            depth,
            diffuse_specular,
            emission,
            position,
            tangents,
            velocity,
        }
    });

    let target = frame.import_image(persistent);

    // lighting pass
    let (_, target) = frame.graphics_subpass("lighting", renderpass, |t| {
        let target = t.load_attachment(&target, vk::AttachmentLoadOp::DontCare);

        t.set_color_attachments(&[&target]);
        t.set_input_attachments(&[
            &gbuffers.normals,
            &gbuffers.diffuse_specular,
            &gbuffers.emission,
            &gbuffers.position,
            &gbuffers.tangents,
            &gbuffers.velocity,
        ]);
        t.set_depth_attachment(&gbuffers.depth);

        t.store_attachment(target, vk::AttachmentStoreOp::Store)
    });

    // present to screen
    frame.present(&target);
}

/*//--------------------------------------------------------------------------------------------------
fn test_frame_0<'ctx>(frame: &mut Frame<'ctx>, persistent: &'ctx mut Image) {
    let (t01, r01) = frame.create_task_on_queue("T01", TaskType::Graphics, 0, |t| {
        t.create_attachment(
            (1024, 1024),
            vk::Format::R16g16b16a16Sfloat,
            AttachmentLoadStore::write_only()
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
}*/

//--------------------------------------------------------------------------------------------------
fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();
    let ctx = &mut app.context;
    let window = &mut app.window;

    // create a persistent image.
    let mut pool = MemoryPool::new(ctx);

    let mut persistent_img = Image::new(
        ctx,
        Dimensions::Dim2D {
            width: 1280,
            height: 720,
        },
        vk::Format::R8g8b8a8Srgb,
        vk::IMAGE_USAGE_SAMPLED_BIT | vk::IMAGE_USAGE_TRANSFER_DST_BIT,
        vk::QUEUE_GRAPHICS_BIT | vk::QUEUE_COMPUTE_BIT,
        Some(pool),
    );

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
                test_frame_deferred_shading(&mut frame, &mut persistent_img);
                frame.submit();
                first = false;
            }
        });
        if should_close {
            break;
        }
    }

    // drop(persistent_img);
}
