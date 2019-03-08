//! Test fixtures for autograph-render and friends.
//! Boilerplate code for creating a window and an OpenGL context with winit/glutin.
use glutin::{Event, EventsLoop, WindowEvent};

pub type Backend = autograph_render_gl::OpenGlBackend;
pub type Renderer = autograph_render::Renderer<Backend>;
pub type Arena<'a> = autograph_render::Arena<'a, Backend>;
pub type Buffer<'a, T> = autograph_render::buffer::Buffer<'a, Backend, T>;
pub type Image<'a> = autograph_render::image::Image<'a, Backend>;
pub type SampledImage<'a> = autograph_render::image::TextureImageView<'a, Backend>;
pub type TypedGraphicsPipeline<'a, T> =
    autograph_render::pipeline::TypedGraphicsPipeline<'a, Backend, T>;
pub type TypedArgumentBlock<'a, T> = autograph_render::pipeline::TypedArgumentBlock<'a, Backend, T>;
pub type TextureImageView<'a> = autograph_render::image::TextureImageView<'a, Backend>;
pub type RenderTargetView<'a> = autograph_render::image::RenderTargetView<'a, Backend>;

pub struct InnerLoop<'a> {
    _renderer: &'a Renderer,
    _arena: &'a Arena<'a>,
    num_frames: Option<usize>,
    events_loop: EventsLoop,
}

impl<'a> InnerLoop<'a> {
    pub fn run<F: FnMut()>(mut self, mut f: F) {
        let mut frames = 0;
        loop {
            let mut should_close = false;
            self.events_loop.poll_events(|event| {
                // event handling
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    } => {
                        should_close = true;
                    }
                    _ => {}
                }
            });

            if should_close {
                break;
            }

            frames += 1;

            if let Some(num_frames) = self.num_frames {
                if frames >= num_frames {
                    break;
                }
            }

            f();
        }
    }
}

pub fn with_test_fixture<F>(title: &str, num_frames: Option<usize>, f: F)
where
    F: FnOnce(&Renderer, &Arena, InnerLoop),
{
    let events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions((640, 480).into());
    let cfg = config::Config::new();
    let (instance, _window) =
        autograph_render_gl::create_instance_and_window(&cfg, &events_loop, window_builder);
    let renderer = Renderer::new(instance);
    let arena = renderer.create_arena();
    f(
        &renderer,
        &arena,
        InnerLoop {
            _renderer: &renderer,
            _arena: &arena,
            num_frames,
            events_loop,
        },
    )
}

#[cfg(test)]
mod tests {
    use crate::with_test_fixture;
    use autograph_render::{
        format::Format,
        image::{ImageUsageFlags, MipmapsOption},
        AliasScope,
    };
    use std::iter;

    fn works() {
        with_test_fixture("test", Some(60), |renderer, arena, innerloop| {
            let img = arena.create_image(
                AliasScope::no_alias(),
                Format::R16G16B16A16_UNORM,
                (640, 480).into(),
                MipmapsOption::One,
                1,
                ImageUsageFlags::SAMPLED,
            );

            innerloop.run(|| {
                let arena = renderer.create_arena();
                let mut cmdbuf = renderer.create_command_buffer();
                cmdbuf.present(0, img, renderer.default_swapchain().unwrap());
                renderer.submit_frame(iter::once(cmdbuf));
            })
        })
    }
}
