// Effect manager — owns all visual effect instances, handles update dispatch
// and render dispatch per scene layer. Adding a new effect requires only
// changes here, not in world.rs or scene.rs.

use super::effects::{AudioEffect, AudioFrame, RenderContext};
use super::effects::beat_rings::BeatRings;
use super::effects::hex_background::HexBackground;
use super::effects::fft_visualizer::FftVisualizer;
use super::effects::grid_lines::GridLines;
use super::effects::fireworks::Fireworks;
use super::effects::fire::Fire;
use super::effects::starfield::Starfield;
use super::effects::aurora::Aurora;
use super::effects::flow_field::{self, FlowField};
use super::effects::fluid::{self, Fluid};
use super::effects::crystal::Crystal;
use super::effects::mandelbrot::Mandelbrot;
use super::effects::themes::{self, EffectFlags, VisualTheme};
use super::particles::ParticleSystem;
use super::drawing::Vertex;
use super::audio_analysis::AudioAnalysis;

pub struct EffectManager {
    pub beat_rings: BeatRings,
    pub hex_background: HexBackground,
    pub fft_vis: FftVisualizer,
    pub grid_lines: GridLines,
    pub fireworks: Fireworks,
    pub fire: Fire,
    pub starfield: Starfield,
    pub aurora: Aurora,
    pub flow_field: FlowField,
    pub fluid: Fluid,
    pub crystal: Crystal,
    pub mandelbrot: Mandelbrot,
    pub particles: ParticleSystem,
    pub flags: EffectFlags,
}

impl EffectManager {
    pub fn new(theme: &VisualTheme) -> Self {
        EffectManager {
            beat_rings: BeatRings::new(theme.rings),
            hex_background: HexBackground::new(theme.hex),
            fft_vis: FftVisualizer::new(theme.fft),
            grid_lines: GridLines::new(theme.grid),
            fireworks: { let mut fw = Fireworks::new(); fw.bursts_only = theme.name == "Debug"; fw },
            fire: Fire::new(),
            starfield: Starfield::new(),
            aurora: Aurora::new(),
            flow_field: FlowField::new(),
            fluid: Fluid::new(),
            crystal: Crystal::new(),
            mandelbrot: Mandelbrot::new(),
            particles: ParticleSystem::new(),
            flags: theme.effects.clone(),
        }
    }

    /// Apply a new theme — recreate parameterized effects, reset state.
    pub fn apply_theme(&mut self, theme: &VisualTheme) {
        self.beat_rings = BeatRings::new(theme.rings);
        self.hex_background = HexBackground::new(theme.hex);
        self.fft_vis = FftVisualizer::new(theme.fft);
        self.grid_lines = GridLines::new(theme.grid);
        self.flags = theme.effects.clone();
        self.particles.particles.clear();
        self.flow_field = FlowField::new();
        self.fluid = Fluid::new();
        self.crystal = Crystal::new();
        self.mandelbrot = Mandelbrot::new();
        self.fireworks.shells_only = false;
        self.fireworks.bursts_only = theme.name == "Debug";
    }

    /// Update all effects from the current audio frame.
    /// `analysis` provides rank resolution. Extra state passed for effects that need it.
    pub fn update(
        &mut self,
        audio_frame: &AudioFrame,
        analysis: &AudioAnalysis,
        bindings: &themes::EffectBindings,
        dt: f64,
        fft_locked: bool,
        fft_lock_hovered: bool,
    ) {
        let ef = &self.flags;

        if ef.beat_rings {
            self.beat_rings.trigger_band = analysis.resolve_rank(bindings.beat_rings);
            self.beat_rings.update(audio_frame);
        }
        if ef.hex_background { self.hex_background.update(audio_frame); }
        self.fft_vis.locked = fft_locked;
        self.fft_vis.lock_hovered = fft_lock_hovered;
        if ef.fft_visualizer { self.fft_vis.update(audio_frame); }
        if ef.grid_lines {
            self.grid_lines.distortion_enabled = ef.grid_distortion;
            self.grid_lines.update(audio_frame);
        }
        if ef.fireworks {
            self.fireworks.trigger_band = Some(analysis.resolve_rank(bindings.fireworks));
            self.fireworks.update(audio_frame);
        }
        if ef.fire { self.fire.update(audio_frame); }
        if ef.starfield { self.starfield.update(audio_frame); }
        if ef.aurora { self.aurora.update(audio_frame); }
        if ef.flow_field {
            self.flow_field.update(audio_frame);
            flow_field::tick_particles(&mut self.flow_field, dt as f32);
        }
        if ef.fluid {
            self.fluid.update(audio_frame);
            fluid::tick_particles(&mut self.fluid, dt as f32);
        }
        if ef.crystal { self.crystal.update(audio_frame); }
        if ef.mandelbrot { self.mandelbrot.update(audio_frame); }
    }

    /// Render background effects (transparent, behind board): fireworks, fire, starfield, aurora, flow_field.
    pub fn render_background(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let ef = &self.flags;
        if ef.fireworks { self.fireworks.render(verts, indices, ctx); }
        if ef.fire { self.fire.render(verts, indices, ctx); }
        if ef.starfield { self.starfield.render(verts, indices, ctx); }
        if ef.aurora { self.aurora.render(verts, indices, ctx); }
        if ef.flow_field { self.flow_field.render(verts, indices, ctx); }
        if ef.fluid { self.fluid.render(verts, indices, ctx); }
        if ef.crystal { self.crystal.render(verts, indices, ctx); }
        if ef.mandelbrot { self.mandelbrot.render(verts, indices, ctx); }
    }

    /// Render grid lines (opaque, board layer).
    pub fn render_grid(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        if self.flags.grid_lines {
            self.grid_lines.render(verts, indices, ctx);
        }
    }

    /// Render dashboard effects (transparent): FFT visualizer, hex background, beat rings.
    pub fn render_dashboard(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        let ef = &self.flags;
        if ef.hex_background { self.hex_background.render(verts, indices, ctx); }
        if ef.fft_visualizer { self.fft_vis.render(verts, indices, ctx); }
        if ef.beat_rings { self.beat_rings.render(verts, indices, ctx); }
    }

    /// Render HUD particles (2D screen-space).
    pub fn render_particles(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        self.particles.render(verts, indices);
    }

    /// Handle track change reset (e.g., firework cooldown).
    pub fn on_track_change(&mut self) {
        self.fireworks.shell_cooldown = 3.0;
    }

    /// Initialize GPU resources for any effects that implement GpuEffect.
    /// Call once after GPU device is available (or on device recreation).
    #[allow(dead_code)]
    pub fn create_gpu_resources(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {
        // No GPU effects registered yet. When an effect is ported to GpuEffect,
        // call its create_gpu_resources here:
        // self.flow_field_gpu.create_gpu_resources(device, queue);
    }

    /// Dispatch compute work for all GPU effects.
    /// Call once per frame before rendering. Returns true if any compute work was submitted.
    pub fn dispatch_compute(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _audio: &AudioFrame) -> bool {
        // No GPU effects registered yet. When an effect is ported to GpuEffect:
        // let mut encoder = device.create_command_encoder(&Default::default());
        // self.flow_field_gpu.compute(&mut encoder, audio);
        // queue.submit(std::iter::once(encoder.finish()));
        // return true;
        false
    }
}
