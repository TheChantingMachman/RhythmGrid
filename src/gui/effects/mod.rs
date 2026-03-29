// Effects interface — trait-based visual effect modules.
// Each effect declares its render pass, updates from audio state,
// and emits geometry into vertex/index buffers.

pub mod beat_rings;
pub mod hex_background;
pub mod fft_visualizer;
pub mod grid_lines;
pub mod themes;
pub mod fireworks;
pub mod fire;
pub mod starfield;
pub mod aurora;
pub mod flow_field;
pub mod fluid;
pub mod crystal;
pub mod mandelbrot;
pub mod pipes;

use super::drawing::Vertex;
use wgpu;

/// Audio state snapshot passed to every effect each frame.
pub struct AudioFrame {
    pub bands: [f32; 7],           // raw FFT energy per band
    pub bands_norm: [f32; 7],      // normalized to own recent peak
    pub peak_bands: [f32; 7],      // slow-decay peak hold
    pub band_beats: [f32; 7],      // per-band beat intensity (1.0 on beat, decays)
    pub centroid: f32,             // spectral centroid 0-1 (dark↔bright)
    pub flux: f32,                 // spectral flux (rate of spectral change)
    pub danger: f32,               // escalation modifier 0-1
    pub dt: f32,                   // frame delta time in seconds
    pub resolved_ranks: [usize; 3], // top 3 bands by energy+confidence (adaptive)
}

/// Rendering context — board geometry and window info.
#[allow(dead_code)]
pub struct RenderContext {
    pub board_width: f32,
    pub board_height: f32,
    pub win_w: f32,
    pub win_h: f32,
    pub window_aspect: f32,
    pub preview_angle: f32,
    pub hud_opacity: f32,
}

/// Which render pass an effect targets.
#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum RenderPass {
    Opaque,
    Transparent,
    Hud,
}

/// A visual effect module driven by audio data (CPU-side geometry generation).
pub trait AudioEffect {
    /// Which render pass this effect's geometry belongs to.
    #[allow(dead_code)]
    fn pass(&self) -> RenderPass;

    /// Update internal state from the current audio frame.
    fn update(&mut self, audio: &AudioFrame);

    /// Emit geometry into vertex/index buffers.
    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext);
}

/// A GPU-driven visual effect that owns its own compute pipeline and storage buffers.
/// Effects implementing this trait run particle simulation / fluid dynamics on the GPU
/// and draw directly from GPU-side buffers — no CPU geometry generation needed.
///
/// Lifecycle:
/// 1. `create_gpu_resources` — called once (or on device change) to allocate pipelines,
///    storage buffers, bind groups. The effect owns all its GPU state.
/// 2. `compute` — called each frame before rendering. Dispatches compute work
///    (particle advection, noise field generation, etc.) using the audio frame for reactivity.
/// 3. `render_gpu` — called during the OIT render pass. Binds its own pipeline/buffers
///    and issues draw calls. The pass is already begun by the caller.
///
/// CPU AudioEffect and GPU GpuEffect coexist — the EffectManager dispatches to the right
/// interface per effect. Effects can be ported from AudioEffect to GpuEffect one at a time.
pub trait GpuEffect {
    /// Allocate GPU resources: compute pipeline, storage buffers, bind groups.
    /// `scene_bgl` is the scene uniform bind group layout (view_proj + camera_pos at group 0).
    fn create_gpu_resources(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene_bgl: &wgpu::BindGroupLayout);

    /// Dispatch compute work for this frame.
    fn compute(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, audio: &AudioFrame);

    /// Issue draw calls into the OIT render pass.
    /// The pass is already begun — set pipeline, bind groups, and draw.
    /// `scene_bg` contains view_proj + camera_pos and should be set at group 0.
    #[allow(dead_code)]
    fn render_gpu<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, scene_bg: &'a wgpu::BindGroup);

    /// Whether GPU resources have been initialized and the effect should use the GPU path.
    fn gpu_active(&self) -> bool;
}
