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

use super::drawing::Vertex;

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

/// A visual effect module driven by audio data.
pub trait AudioEffect {
    /// Which render pass this effect's geometry belongs to.
    #[allow(dead_code)]
    fn pass(&self) -> RenderPass;

    /// Update internal state from the current audio frame.
    fn update(&mut self, audio: &AudioFrame);

    /// Emit geometry into vertex/index buffers.
    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext);
}


