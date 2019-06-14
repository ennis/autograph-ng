#![feature(proc_macro_hygiene)]
use autograph_imgui::ImGuiRenderer;
use autograph_api::format::Format;
use autograph_api_boilerplate::{App, Event, KeyboardInput, WindowEvent};
use imgui::{self, FontGlyphRange, ImGui};
use log::info;
use std::{env, time};

// Change the default font because why not
//const FONT: &[u8] = include_bytes!("nokiafc22.ttf");
const FONT: &[u8] = include_bytes!("ChiKareGo2.ttf");
const FONT_SIZE: f32 = 15.0;

pub struct ImGuiContext {
    app_hidpi_factor: f64,
    imgui: imgui::ImGui,
    last_frame_time: time::Instant,
}

impl ImGuiContext {
    pub fn new(app_hidpi_factor: f64) -> ImGuiContext {
        let mut imgui = imgui::ImGui::init();
        imgui
            .fonts()
            .add_font(FONT, FONT_SIZE, &FontGlyphRange::default());
        imgui_winit_support::configure_keys(&mut imgui);
        ImGuiContext {
            app_hidpi_factor,
            imgui,
            last_frame_time: time::Instant::now(),
        }
    }

    pub fn handle_event(&mut self, window: &winit::Window, event: &winit::Event) {
        imgui_winit_support::handle_event(
            &mut self.imgui,
            event,
            window.get_hidpi_factor(),
            self.app_hidpi_factor,
        );
    }

    pub fn frame(&mut self, window: &winit::Window) -> imgui::Ui {
        let frame_size =
            imgui_winit_support::get_frame_size(window, self.app_hidpi_factor).unwrap();
        let elapsed = self.last_frame_time.elapsed();
        let delta_time =
            (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1_000_000_000.0);
        self.last_frame_time = time::Instant::now();
        self.imgui.frame(frame_size, delta_time as f32)
    }

    pub fn imgui(&mut self) -> &mut ImGui {
        &mut self.imgui
    }
}

fn test_ui(ui: &mut imgui::Ui) {
    use imgui::*;

    ui.window(im_str!("Hello world"))
        .size((300.0, 100.0), ImGuiCond::FirstUseEver)
        .build(|| {
            ui.text(im_str!("Hello world!"));
            ui.text(im_str!("こんにちは世界！"));
            ui.separator();
            let mouse_pos = ui.imgui().mouse_pos();
            ui.text(im_str!(
                "Mouse Position: ({:.1},{:.1})",
                mouse_pos.0,
                mouse_pos.1
            ));
        });
    let mut demo = true;
    ui.show_demo_window(&mut demo);
}

#[test]
fn test_imgui() {
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).unwrap();

    let app = App::new();
    let r = app.renderer();

    // create imgui
    let mut imgui = ImGuiContext::new(1.0);

    'outer: loop {
        let default_swapchain = r.default_swapchain().unwrap();
        let (w, h) = default_swapchain.size();
        let arena_1 = r.create_arena();
        let color_buffer =
            arena_1.create_unaliasable_render_target(Format::R8G8B8A8_SRGB, (w, h), 1);
        let mut imgui_renderer =
            ImGuiRenderer::new(&arena_1, imgui.imgui(), color_buffer, (w, h).into());

        'inner: loop {
            //----------------------------------------------------------------------------------
            // handle events
            let should_close = app.poll_events(|event| {
                imgui.handle_event(app.window(), &event);
                match event {
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
                    }
                    _ => {}
                }
            });

            let mut ui = imgui.frame(app.window());
            test_ui(&mut ui);

            let arena_frame = r.create_arena();
            let mut cmdbuf = r.create_command_buffer();
            cmdbuf.clear_image(0x0, color_buffer, &[0.0, 0.2, 0.8, 1.0]);
            imgui_renderer.render(&mut cmdbuf, 0x0, &arena_frame, ui);
            cmdbuf.present(0x0, color_buffer, default_swapchain);
            r.submit_frame(vec![cmdbuf]);

            if should_close {
                break 'outer;
            }

            let (new_w, new_h) = default_swapchain.size();

            // don't resize if new size is null in one dimension, as it will
            // cause create_framebuffer to fail.
            if (new_w, new_h) != (w, h) && new_w != 0 && new_h != 0 {
                break 'inner;
            }
        }
    }
}
