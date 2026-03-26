// Aurora effect — flowing ribbon bands that converge toward a slowly drifting
// vanishing point below the board. Each ribbon has independent brightness
// cycling, edge fade, and perspective convergence.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use crate::gui::drawing::Vertex;

struct Ribbon {
    base_y: f32,
    phase: f32,
    freq1: f32,
    freq2: f32,
    amp1: f32,
    amp2: f32,
    speed1: f32,
    speed2: f32,
    width: f32,
    hue_offset: f32,
    base_brightness: f32,
    // Independent brightness cycle
    bright_phase: f32,
    bright_freq: f32,  // how fast this ribbon fades in/out independently
}

pub struct Aurora {
    ribbons: Vec<Ribbon>,
    time: f32,
    intensity: f32,
    centroid: f32,
    // Vanishing point — drifts very slowly in the SW-SE arc below the board
    vp_x: f32,
    vp_y: f32,
    vp_target_x: f32,
    vp_drift_timer: f32,
    rng: u64,
}

const RIBBON_SEGMENTS: usize = 40;
const CENTER_X: f32 = 5.0;

fn rng_next(rng: &mut u64) -> f32 {
    *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*rng >> 33) as f32) / (u32::MAX as f32 / 2.0)
}

impl Aurora {
    pub fn new() -> Self {
        let ribbons = vec![
            Ribbon {
                base_y: -4.0, phase: 0.0,
                freq1: 0.8, freq2: 2.1, amp1: 1.2, amp2: 0.4,
                speed1: 0.3, speed2: 0.7, width: 5.4,
                hue_offset: 0.0, base_brightness: 1.0,
                bright_phase: 0.0, bright_freq: 0.08,
            },
            Ribbon {
                base_y: -7.0, phase: 1.5,
                freq1: 0.6, freq2: 1.8, amp1: 1.5, amp2: 0.5,
                speed1: 0.25, speed2: 0.55, width: 6.6,
                hue_offset: 0.15, base_brightness: 0.8,
                bright_phase: 1.2, bright_freq: 0.12,
            },
            Ribbon {
                base_y: -10.0, phase: 3.0,
                freq1: 1.0, freq2: 2.5, amp1: 1.0, amp2: 0.35,
                speed1: 0.35, speed2: 0.8, width: 4.5,
                hue_offset: 0.3, base_brightness: 0.9,
                bright_phase: 2.8, bright_freq: 0.06,
            },
            Ribbon {
                base_y: -13.0, phase: 4.5,
                freq1: 0.5, freq2: 1.5, amp1: 1.8, amp2: 0.6,
                speed1: 0.2, speed2: 0.45, width: 7.5,
                hue_offset: 0.5, base_brightness: 0.6,
                bright_phase: 4.0, bright_freq: 0.1,
            },
            Ribbon {
                base_y: -16.0, phase: 2.2,
                freq1: 0.9, freq2: 2.3, amp1: 1.3, amp2: 0.45,
                speed1: 0.28, speed2: 0.65, width: 1.6,
                hue_offset: 0.7, base_brightness: 0.7,
                bright_phase: 5.5, bright_freq: 0.07,
            },
        ];

        Aurora {
            ribbons,
            time: 0.0,
            intensity: 0.0,
            centroid: 0.5,
            vp_x: CENTER_X,
            vp_y: -28.0,
            vp_target_x: CENTER_X + 2.0,
            vp_drift_timer: 0.0,
            rng: 0xA020A042,
        }
    }

    fn aurora_color(hue: f32, brightness: f32) -> [f32; 3] {
        let h = hue.fract();
        if h < 0.25 {
            let t = h / 0.25;
            [0.1 * brightness, (0.8 + t * 0.2) * brightness, (0.3 + t * 0.5) * brightness]
        } else if h < 0.5 {
            let t = (h - 0.25) / 0.25;
            [(0.1 + t * 0.3) * brightness, (1.0 - t * 0.5) * brightness, (0.8 + t * 0.2) * brightness]
        } else if h < 0.75 {
            let t = (h - 0.5) / 0.25;
            [(0.4 + t * 0.4) * brightness, (0.5 - t * 0.2) * brightness, (1.0 - t * 0.2) * brightness]
        } else {
            let t = (h - 0.75) / 0.25;
            [(0.8 - t * 0.7) * brightness, (0.3 + t * 0.5) * brightness, (0.8 - t * 0.5) * brightness]
        }
    }
}

impl AudioEffect for Aurora {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;

        // Intensity follows mids + upper mids
        let raw = audio.bands_norm[2] * 0.3 + audio.bands_norm[3] * 0.4 + audio.bands_norm[4] * 0.3;
        self.intensity += (raw - self.intensity) * 1.5 * audio.dt;

        // Centroid drives color shift
        self.centroid += (audio.centroid - self.centroid) * 0.5 * audio.dt;

        // Independent brightness cycling per ribbon
        for ribbon in &mut self.ribbons {
            ribbon.bright_phase += audio.dt * ribbon.bright_freq;
        }

        // Vanishing point drift — very slow, picks a new target every 30-60s
        self.vp_drift_timer -= audio.dt;
        if self.vp_drift_timer <= 0.0 {
            // New target in the SW-SE arc (x range: -5 to 15, centered on board)
            self.vp_target_x = CENTER_X + rng_next(&mut self.rng) * 10.0;
            self.vp_drift_timer = 30.0 + rng_next(&mut self.rng).abs() * 30.0;
        }
        // Creep toward target
        self.vp_x += (self.vp_target_x - self.vp_x) * 0.02 * audio.dt;
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let n = [0.0f32, 0.0, 1.0];
        let z_base = -2.5;

        let x_start = -15.0;
        let x_end = 25.0;
        let x_span = x_end - x_start;
        let x_center = (x_start + x_end) * 0.5;

        for ribbon in &self.ribbons {
            // Independent brightness: slow sine cycle, 0.2 to 1.0
            let bright_cycle = 0.2 + 0.8 * ((ribbon.bright_phase * std::f32::consts::TAU).sin() * 0.5 + 0.5);
            let intensity_mod = (0.3 + self.intensity * 0.7) * bright_cycle;
            let base_alpha = 0.07 * ribbon.base_brightness * intensity_mod;

            for seg in 0..RIBBON_SEGMENTS {
                let t0 = seg as f32 / RIBBON_SEGMENTS as f32;
                let t1 = (seg + 1) as f32 / RIBBON_SEGMENTS as f32;

                // Raw x positions
                let raw_x0 = x_start + t0 * x_span;
                let raw_x1 = x_start + t1 * x_span;

                // Perspective convergence — gentle pull toward vanishing point
                let depth_factor = ((ribbon.base_y - self.vp_y) / 25.0).clamp(0.0, 1.0) * 0.15;
                let edge0 = ((raw_x0 - x_center) / (x_span * 0.5)).abs();
                let edge1 = ((raw_x1 - x_center) / (x_span * 0.5)).abs();
                let converge0 = edge0 * edge0 * depth_factor;
                let converge1 = edge1 * edge1 * depth_factor;
                let x0 = raw_x0 + (self.vp_x - raw_x0) * converge0;
                let x1 = raw_x1 + (self.vp_x - raw_x1) * converge1;

                // Subtle z-depth and foreshortening at edges
                let z0 = z_base - converge0 * 1.5;
                let z1 = z_base - converge1 * 1.5;
                let foreshorten0 = 1.0 - converge0 * 0.3;
                let foreshorten1 = 1.0 - converge1 * 0.3;

                // Dual-sine undulation
                let wave0 = (t0 * ribbon.freq1 * std::f32::consts::TAU + self.time * ribbon.speed1 + ribbon.phase).sin() * ribbon.amp1
                          + (t0 * ribbon.freq2 * std::f32::consts::TAU + self.time * ribbon.speed2 + ribbon.phase * 1.7).sin() * ribbon.amp2;
                let wave1 = (t1 * ribbon.freq1 * std::f32::consts::TAU + self.time * ribbon.speed1 + ribbon.phase).sin() * ribbon.amp1
                          + (t1 * ribbon.freq2 * std::f32::consts::TAU + self.time * ribbon.speed2 + ribbon.phase * 1.7).sin() * ribbon.amp2;

                let amp_mod = 1.0 + self.intensity * 1.5;
                let y0_center = ribbon.base_y + wave0 * amp_mod;
                let y1_center = ribbon.base_y + wave1 * amp_mod;

                let base_half_w = ribbon.width * 0.5 * (0.8 + self.intensity * 0.4);
                let half_w0 = base_half_w * foreshorten0;
                let half_w1 = base_half_w * foreshorten1;

                // Edge fade: alpha drops to 0 at ribbon endpoints
                let edge_fade0 = (t0 * 4.0).min(1.0).min((1.0 - t0) * 4.0); // fade over first/last 25%
                let edge_fade1 = (t1 * 4.0).min(1.0).min((1.0 - t1) * 4.0);
                let seg_alpha0 = base_alpha * edge_fade0;
                let seg_alpha1 = base_alpha * edge_fade1;

                // Color
                let hue = ribbon.hue_offset + self.centroid * 0.5 + t0 * 0.3;
                let rgb = Self::aurora_color(hue, intensity_mod);

                // Top edge → transparent
                let top0 = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 0.0];
                let top1 = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 0.0];
                // Center → brightest
                let mid0 = [rgb[0] * 1.3, rgb[1] * 1.3, rgb[2] * 1.3, seg_alpha0];
                let mid1 = [rgb[0] * 1.3, rgb[1] * 1.3, rgb[2] * 1.3, seg_alpha1];
                // Bottom edge → transparent
                let bot0 = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 0.0];
                let bot1 = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 0.0];

                // Top half: top edge (transparent) → center (bright)
                let base = verts.len() as u32;
                verts.push(Vertex { position: [x0, y0_center + half_w0, z0], normal: n, color: top0, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x1, y1_center + half_w1, z1], normal: n, color: top1, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x1, y1_center, z1], normal: n, color: mid1, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x0, y0_center, z0], normal: n, color: mid0, uv: [0.0, 0.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

                // Bottom half: center (bright) → bottom edge (transparent)
                let base = verts.len() as u32;
                verts.push(Vertex { position: [x0, y0_center, z0], normal: n, color: mid0, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x1, y1_center, z1], normal: n, color: mid1, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x1, y1_center - half_w1, z1], normal: n, color: bot1, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [x0, y0_center - half_w0, z0], normal: n, color: bot0, uv: [0.0, 0.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
