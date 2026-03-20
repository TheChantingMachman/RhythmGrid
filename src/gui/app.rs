// Winit application handler — event loop and window management.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use rhythm_grid::input;

use super::input_bridge::winit_to_rg;
use super::renderer::{GpuState, Uniforms};
use super::theme::THEME;
use super::world::GameWorld;

pub struct App {
    pub window: Option<Arc<Window>>,
    pub gpu: Option<GpuState>,
    pub world: GameWorld,
    pub pending_resize: Option<(u32, u32)>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("RhythmGrid")
            .with_inner_size(winit::dpi::LogicalSize::new(THEME.win_w, THEME.win_h))
            .with_min_inner_size(winit::dpi::LogicalSize::new(THEME.win_w / 2, THEME.win_h / 2));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let gpu = GpuState::new(window.clone());
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                self.pending_resize = Some((new_size.width, new_size.height));
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(code), state: ElementState::Pressed, .. }, .. } => {
                let rg_key = winit_to_rg(code);
                if let Some(action) = input::map_key(rg_key) {
                    self.world.handle_action(action);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some((w, h)) = self.pending_resize.take() {
                    if let Some(gpu) = &mut self.gpu {
                        gpu.resize(w, h);
                    }
                }
                self.world.tick();
                let ((sv, si), (hv, hi)) = self.world.build_scene_and_hud();
                if sv.len() > 0 && si.len() > 0 {
                    // Print once
                    static ONCE: std::sync::Once = std::sync::Once::new();
                    ONCE.call_once(|| {
                        eprintln!("Scene: {} verts, {} indices | HUD: {} verts, {} indices",
                            sv.len(), si.len(), hv.len(), hi.len());
                        eprintln!("First scene vert: {:?}", sv[0]);
                    });
                }
                if let Some(gpu) = &self.gpu {
                    gpu.update_uniforms(&self.world.compute_uniforms());
                    gpu.render(&sv, &si, &hv, &hi);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
