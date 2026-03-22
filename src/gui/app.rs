// Winit application handler — event loop and window management.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use rhythm_grid::input;

use super::input_bridge::winit_to_rg;
use super::renderer::GpuState;
use super::theme::THEME;
use super::world::GameWorld;

pub struct App {
    pub window: Option<Arc<Window>>,
    pub gpu: Option<GpuState>,
    pub world: GameWorld,
    pub pending_resize: Option<(u32, u32)>,
    pub resize_debounce: std::time::Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let attrs = Window::default_attributes()
            .with_title("RhythmGrid")
            .with_inner_size(winit::dpi::LogicalSize::new(THEME.win_w, THEME.win_h))
            .with_min_inner_size(winit::dpi::LogicalSize::new(800, 600));
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
                self.resize_debounce = std::time::Instant::now();
            }
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(code), state: ElementState::Pressed, .. }, .. } => {
                self.world.demo_idle_timer = 0.0; // any key resets idle
                use winit::keyboard::KeyCode as K;
                match code {
                    K::F1 => self.world.cycle_theme(),
                    K::KeyN => self.world.skip_track(),
                    K::ShiftLeft | K::ShiftRight => self.world.hold_piece(),
                    K::Equal | K::NumpadAdd => self.world.adjust_volume(0.05),
                    K::Minus | K::NumpadSubtract => self.world.adjust_volume(-0.05),
                    _ => {
                        let rg_key = winit_to_rg(code);
                        if let Some(action) = input::map_key(rg_key) {
                            self.world.handle_action(action);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.world.cursor_pos = [position.x as f32, position.y as f32];
                self.world.on_mouse_activity();
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                // Refresh hover state before checking click — avoids stale hover from previous frame
                if let Some(gpu) = &self.gpu {
                    let uniforms = self.world.compute_uniforms(gpu.aspect_ratio());
                    self.world.update_button_rects(&uniforms, gpu.aspect_ratio());
                }
                self.world.handle_click();
            }
            WindowEvent::RedrawRequested => {
                // Debounce resize — only apply after 50ms of no resize events
                if self.pending_resize.is_some()
                    && self.resize_debounce.elapsed() >= std::time::Duration::from_millis(50)
                {
                    if let Some((w, h)) = self.pending_resize.take() {
                        if let Some(gpu) = &mut self.gpu {
                            gpu.resize(w, h);
                        }
                    }
                }
                self.world.tick();
                if let Some(gpu) = &self.gpu {
                    self.world.window_aspect = gpu.aspect_ratio();
                    let (sw, sh) = gpu.size();
                    self.world.window_size = [sw, sh];
                    let uniforms = self.world.compute_uniforms(gpu.aspect_ratio());
                    self.world.update_button_rects(&uniforms, gpu.aspect_ratio());
                    gpu.update_uniforms(&uniforms);
                }
                let ((ov, oi), (tv, ti), (hv, hi)) = self.world.build_scene_and_hud();
                if let Some(gpu) = &self.gpu {
                    gpu.render(&ov, &oi, &tv, &ti, &hv, &hi);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
