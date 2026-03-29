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
        let mut attrs = Window::default_attributes()
            .with_title("RhythmGrid")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.world.saved_window_width,
                self.world.saved_window_height,
            ))
            .with_min_inner_size(winit::dpi::LogicalSize::new(800, 600));
        if let (Some(x), Some(y)) = (self.world.saved_window_x, self.world.saved_window_y) {
            attrs = attrs.with_position(winit::dpi::LogicalPosition::new(x, y));
        }
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let gpu = GpuState::new(window.clone());
        // Initialize GPU resources for effects that use compute shaders
        self.world.effects.create_gpu_resources(gpu.device(), gpu.queue(), gpu.scene_bgl());
        self.gpu = Some(gpu);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // Save window position on close
                if let Some(w) = &self.window {
                    if let Ok(pos) = w.outer_position() {
                        self.world.saved_window_x = Some(pos.x);
                        self.world.saved_window_y = Some(pos.y);
                    }
                }
                self.world.save_settings();
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.pending_resize = Some((new_size.width, new_size.height));
                self.resize_debounce = std::time::Instant::now();
                // Track logical size for persistence
                if let Some(w) = &self.window {
                    let scale = w.scale_factor();
                    self.world.logical_window_size = [
                        (new_size.width as f64 / scale) as u32,
                        (new_size.height as f64 / scale) as u32,
                    ];
                }
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
                        // Menu navigation takes priority over game actions
                        if !self.world.handle_menu_key(&code) {
                            let rg_key = winit_to_rg(code);
                            if let Some(action) = input::map_key(rg_key) {
                                self.world.handle_action(action);
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.world.cursor_pos = [position.x as f32, position.y as f32];
                self.world.on_mouse_activity();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as i32,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.y / 30.0) as i32,
                };
                self.world.handle_scroll(lines);
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
                        self.world.save_settings(); // persist new size
                    }
                }
                if let Some(gpu) = &self.gpu {
                    self.world.window_aspect = gpu.aspect_ratio();
                    let (sw, sh) = gpu.size();
                    self.world.window_size = [sw, sh];
                }
                self.world.tick();
                if let Some(gpu) = &self.gpu {
                    let uniforms = self.world.compute_uniforms(gpu.aspect_ratio());
                    self.world.update_button_rects(&uniforms, gpu.aspect_ratio());
                    self.world.update_track_queue_rects();
                    gpu.update_uniforms(&uniforms);
                    gpu.set_color_grade(self.world.color_grade);
                }
                // Dispatch GPU compute for any GpuEffect instances
                if let Some(gpu) = &self.gpu {
                    self.world.effects.dispatch_compute(gpu.device(), gpu.queue(), &self.world.audio_frame);
                }
                // Mandelbrot GPU background
                if let Some(gpu) = &mut self.gpu {
                    let mb = &self.world.effects.mandelbrot;
                    let active = self.world.effects.flags.mandelbrot && mb.use_gpu();
                    gpu.mandelbrot_active = active;
                    if active {
                        gpu.update_mandelbrot(&mb.gpu_uniforms(gpu.aspect_ratio()));
                    }
                }
                let ((ov, oi), (tv, ti), (hv, hi)) = self.world.build_scene_and_hud();
                let gpu_draw = self.world.effects.flow_field.gpu_draw_cmd();
                if let Some(gpu) = &mut self.gpu {
                    gpu.warp_intensity = self.world.journey_transition.warp_intensity();
                    gpu.render(&ov, &oi, &tv, &ti, &hv, &hi, gpu_draw.as_ref());
                }
                if self.world.should_quit {
                    self.world.save_settings();
                    event_loop.exit();
                    return;
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}
