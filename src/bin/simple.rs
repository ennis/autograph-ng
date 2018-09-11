extern crate gfx2;
extern crate ash;

use std::env;

use gfx2::vk;
use gfx2::window::*;
use gfx2::frame::*;
use gfx2::texture::get_texture_mip_map_count;

fn downsample(frame: &mut Frame, input: ImageRef) -> ImageRef
{
    let create_info = frame.get_image_create_info(&input).clone();
    let (w,h) = (create_info.extent.width, create_info.extent.height);
    let count = get_texture_mip_map_count(w,h);

    let mut r_last = input;
    let mut cur_w = w;
    let mut cur_h = h;
    for i in 0..count {
        let t = frame.create_task("downsample");
        frame.image_sample_dependency(t, &r_last);
        cur_w /= 2;
        cur_h /= 2;
        let r_target = frame.create_image_2d((cur_w, cur_h), vk::Format::R16g16b16a16Sfloat);
        r_last = frame.color_attachment_dependency(t, 0, &r_target);
    }

    r_last
}

fn main() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR"));

    // this creates an event loop, a window, context, and a swapchain associated to the window.
    let mut app = App::new();
    let ctx = &mut app.context;
    let window = &mut app.window;

    // create a persistent framebuffer texture.
    //let mut persistent_tex = ctx.create_texture(unimplemented!());
    // create a persistent depth texture.

    let mut first = true;
    loop {
        let mut should_close = false;
        app.events_loop.poll_events(|event| {
            // event handling
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    should_close = true;
                },
                // if resize, then delete persistent resources and re-create
                _ => ()
            }


            if first {
                let mut frame = ctx.new_frame();
                // initial task
                let t_init = frame.create_task("init");
                let r_color_a = frame.create_image_2d((1024, 1024), vk::Format::R16g16b16a16Sfloat);
                let r_color_b = frame.create_image_2d((1024, 1024), vk::Format::R16g16b16a16Sfloat);
                // render to target
                let t_render = frame.create_task("render");
                let r_color_a = frame.color_attachment_dependency(t_render, 0, &r_color_a);
                let r_color_b = frame.color_attachment_dependency(t_render, 1, &r_color_b);
                // downsample one
                let r_color_b = downsample(&mut frame, r_color_b);
                // post-process
                let t_postproc = frame.create_task("postproc");
                frame.image_sample_dependency(t_postproc, &r_color_a);
                frame.image_sample_dependency(t_postproc, &r_color_b);
                let r_output = frame.create_image_2d((1024, 1024), vk::Format::R8g8b8a8Srgb);
                frame.color_attachment_dependency(t_postproc, 0, &r_output);
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

            // Graph:
            // Node = pass type (graphics or compute), callback function
            // callback function parameters:
            //  - command buffer, within a subpass instance initialized with the correct attachments
            //  - container object that holds all requested resources in their correct state (all images, sampled images, buffer, storage buffer, uniform buffers, etc.)
            // The callback just has to submit the draw command.
            // Edge = dependency
            // - toposort
            // - check for concurrent read/write hazards
            // - infer usage of resources from graphs (mutate graph)
            // - schedule renderpasses
            // - reorder to minimize layout transitions
            // - allocate resources
            // - group in render passes (draw commands with the same attachments; notably: chains without `sample` dependencies)
            //      - new dependency type: attachment input
            //      - at least one subpass for each different attachment?
            //      - minimize the number of attachments
            //      - a sample dependency always breaks the render pass
            // - heuristic for renderpasses:
            //      - schedule a pass (starting from the first one)
            //      - follow all output attachments
            //      - if no successor has a sample-dependency, and NO OTHER ATTACHMENTS, then merge the successors into the renderpass
            //      - schedule_renderpasses()
            //      - create dependencies between subpasses (e.g. output-to-input attachment)
            //      - user-provided hints?
            // - schedule renderpasses (dependencies between renderpasses: e.g. layout transitions)
            // - insert memory barriers
            // - insert layout transitions
            //
            // Various graphs:
            // - initial graph
            // - graph with render passes
            // - graph with render passes and explicit layout transitions


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
        if should_close { break }
    }

    // delete persistent textures.
    // context.release(persistent_tex);
}
