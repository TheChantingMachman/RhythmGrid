// Effects interface — trait-based visual effect modules.
// Each effect declares its render pass, updates from audio state,
// and emits geometry into vertex/index buffers.

pub mod beat_rings;
pub mod hex_background;
pub mod fft_visualizer;
pub mod grid_lines;

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
pub enum RenderPass {
    Opaque,
    Transparent,
    Hud,
}

/// A visual effect module driven by audio data.
pub trait AudioEffect {
    /// Which render pass this effect's geometry belongs to.
    fn pass(&self) -> RenderPass;

    /// Update internal state from the current audio frame.
    fn update(&mut self, audio: &AudioFrame);

    /// Emit geometry into vertex/index buffers.
    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext);
}

/// A collection of effects that defines a visual theme.
pub struct Theme {
    pub effects: Vec<Box<dyn AudioEffect>>,
}

impl Theme {
    pub fn update_all(&mut self, audio: &AudioFrame) {
        for effect in &mut self.effects {
            effect.update(audio);
        }
    }

    pub fn render_pass(&self, pass: RenderPass, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, ctx: &RenderContext) {
        for effect in &self.effects {
            if effect.pass() == pass {
                effect.render(verts, indices, ctx);
            }
        }
    }
}
