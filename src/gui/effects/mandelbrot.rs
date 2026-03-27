// Mandelbrot fractal zoom — infinitely zooming corridor effect.
// Rendered as a fullscreen quad with per-pixel fractal iteration in the fragment shader.
// Audio-reactive: zoom speed, color cycling, iteration count driven by audio energy.
// Implemented as a CPU-side effect that emits a screen-filling quad with uniforms
// encoded in vertex colors/UVs, since our effect system outputs vertex geometry.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::Vertex;

pub struct Mandelbrot {
    time: f32,
    zoom: f64,        // current zoom level (grows exponentially)
    energy: f32,
    centroid: f32,
    // Zoom target — Seahorse Valley, rich fractal detail
    target_re: f64,
    target_im: f64,
}

const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

impl Mandelbrot {
    pub fn new() -> Self {
        Mandelbrot {
            time: 0.0,
            zoom: 1.0,
            energy: 0.0,
            centroid: 0.5,
            // Seahorse Valley — classic infinite zoom point
            target_re: -0.743643887037158704752191506114774,
            target_im: 0.131825904205311970493132056385139,
        }
    }
}

impl AudioEffect for Mandelbrot {
    fn pass(&self) -> RenderPass { RenderPass::Transparent }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;
        let target_energy = audio.bands.iter().sum::<f32>() / 7.0;
        self.energy += (target_energy - self.energy) * 3.0 * audio.dt;
        self.centroid += (audio.centroid - self.centroid) * 2.0 * audio.dt;

        // Zoom speed — base rate + audio boost
        let zoom_speed = 0.3 + self.energy as f64 * 0.5;
        self.zoom *= 1.0 + zoom_speed * audio.dt as f64;

        // Reset zoom after it gets extremely deep (precision limit)
        if self.zoom > 1e13 {
            self.zoom = 1.0;
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        // Since we can't run a custom shader through the AudioEffect trait,
        // we approximate the Mandelbrot set by computing it on CPU and rendering
        // as a grid of colored quads. This is a prototype — GPU shader version
        // would be much higher resolution.

        let grid_w = 160;
        let grid_h = 120;
        let max_iter = 48 + (self.energy * 32.0) as u32;

        // Viewport in complex plane
        let scale = 2.0 / self.zoom;
        let aspect = 1.5; // approximate screen aspect
        let x_min = self.target_re - scale * aspect;
        let x_max = self.target_re + scale * aspect;
        let y_min = self.target_im - scale;
        let y_max = self.target_im + scale;

        // World-space bounds for the quad grid (fills behind the board)
        let world_x0 = -15.0f32;
        let world_x1 = 25.0f32;
        let world_y0 = 5.0f32;
        let world_y1 = -25.0f32;
        let z = -4.0f32;

        let cell_w = (world_x1 - world_x0) / grid_w as f32;
        let cell_h = (world_y1 - world_y0) / grid_h as f32;

        for gy in 0..grid_h {
            for gx in 0..grid_w {
                // Map grid cell to complex plane
                let cx = x_min + (gx as f64 / grid_w as f64) * (x_max - x_min);
                let cy = y_min + (gy as f64 / grid_h as f64) * (y_max - y_min);

                // Mandelbrot iteration
                let mut zr = 0.0f64;
                let mut zi = 0.0f64;
                let mut iter = 0u32;
                while iter < max_iter {
                    let zr2 = zr * zr;
                    let zi2 = zi * zi;
                    if zr2 + zi2 > 4.0 { break; }
                    zi = 2.0 * zr * zi + cy;
                    zr = zr2 - zi2 + cx;
                    iter += 1;
                }

                if iter == max_iter { continue; } // inside the set — leave black/empty

                // Color based on iteration count + audio centroid for palette shift
                let t = iter as f32 / max_iter as f32;
                let hue = t * 3.0 + self.time * 0.2 + self.centroid;
                let r = (0.5 + 0.5 * (hue * 6.28).sin()).clamp(0.0, 1.0);
                let g = (0.5 + 0.5 * ((hue + 0.33) * 6.28).sin()).clamp(0.0, 1.0);
                let b = (0.5 + 0.5 * ((hue + 0.67) * 6.28).sin()).clamp(0.0, 1.0);
                let brightness = 0.3 + t * 0.7;
                let color = [r * brightness, g * brightness, b * brightness, 0.9];

                let px = world_x0 + gx as f32 * cell_w;
                let py = world_y0 + gy as f32 * cell_h;

                let base = verts.len() as u32;
                verts.push(Vertex { position: [px, py, z], normal: NORMAL, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [px + cell_w, py, z], normal: NORMAL, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [px + cell_w, py + cell_h, z], normal: NORMAL, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [px, py + cell_h, z], normal: NORMAL, color, uv: [0.0, 0.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}
