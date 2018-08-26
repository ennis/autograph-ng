extern crate gfx2;

use gfx2::window::*;

fn main() {
    // this creates an event loop, a window, and a context.
    let mut app = App::new();
    let ctx = &mut app.context;
    let window = &mut app.window;

    // create a persistent framebuffer texture.
    //let mut persistent_tex = ctx.create_texture(unimplemented!());

    loop {
        let mut should_close = false;
        app.events_loop.poll_events(|event| {
            // event handling
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    should_close = true;
                },
                _ => ()
            }

            // create frame
            //let mut frame = ctx.new_frame();
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
}
