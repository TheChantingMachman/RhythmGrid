// Mandelbrot fractal zoom — infinitely zooming corridor effect.
// Rendered as a fullscreen quad with per-pixel fractal iteration in the fragment shader.
// Audio-reactive: zoom speed, color cycling, iteration count driven by audio energy.
// Implemented as a CPU-side effect that emits a screen-filling quad with uniforms
// encoded in vertex colors/UVs, since our effect system outputs vertex geometry.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::Vertex;
use super::super::renderer::MandelbrotUniforms;

// Color palettes: (num_colors, [[r,g,b,0]; 6])
const PALETTES: [(u32, [[f32; 4]; 6]); 3] = [
    // Red / White / Black
    (3, [
        [1.0, 0.1, 0.1, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0; 4], [0.0; 4], [0.0; 4],
    ]),
    // Blue / White / Black
    (3, [
        [0.1, 0.2, 1.0, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0; 4], [0.0; 4], [0.0; 4],
    ]),
    // Neon green / Black / White
    (3, [
        [0.2, 1.0, 0.2, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [0.0; 4], [0.0; 4], [0.0; 4],
    ]),
];

// Interesting zoom targets on the Mandelbrot set boundary
const ZOOM_TARGETS: [(f32, f32); 8] = [
    (-0.7436439, 0.1318259),   // Seahorse Valley — spirals
    (-0.1011, 0.9563),         // Elephant Valley — trunk shapes
    (-0.0886, 0.654),          // Triple spiral
    (-1.315181, 0.073476),     // Lightning branches
    (-0.745428, 0.186009),     // Mini-brot (miniature copy)
    (-0.748, 0.1),             // Spiral galaxy
    (-0.16, 1.0405),           // Tendrils
    (-1.25066, 0.02012),       // Needle — thin branching
];

// Verlet ragdoll point
struct RagPoint {
    x: f32, y: f32,
    ox: f32, oy: f32, // old position (verlet integration)
}

// Stick figure: 9 joints connected by 8 bones
// 0=head, 1=neck, 2=l_elbow, 3=l_hand, 4=r_elbow, 5=r_hand,
// 6=hip, 7=l_knee, 8=l_foot, 9=r_knee, 10=r_foot
#[allow(dead_code)]
const NUM_JOINTS: usize = 11;
const BONES: [(usize, usize, f32); 10] = [
    (0, 1, 0.5),   // head-neck
    (1, 6, 1.0),   // neck-hip (torso)
    (1, 2, 0.6),   // neck-l_elbow
    (2, 3, 0.6),   // l_elbow-l_hand
    (1, 4, 0.6),   // neck-r_elbow
    (4, 5, 0.6),   // r_elbow-r_hand
    (6, 7, 0.7),   // hip-l_knee
    (7, 8, 0.7),   // l_knee-l_foot
    (6, 9, 0.7),   // hip-r_knee
    (9, 10, 0.7),  // r_knee-r_foot
];

pub struct Mandelbrot {
    time: f32,
    zoom: f32,
    energy: f32,
    centroid: f32,
    target_re: f32,
    target_im: f32,
    #[allow(dead_code)]
    rng_state: u64,
    target_index: usize,
    palette_index: usize,
    // Ragdoll stick figure
    joints: Vec<RagPoint>,
    drift_x: f32,   // slow horizontal meander
    drift_y: f32,
    tumble: f32,     // gentle rotation applied as perturbation
}

impl Mandelbrot {
    pub fn new() -> Self {
        let (re, im) = ZOOM_TARGETS[0];
        Mandelbrot {
            time: 0.0,
            zoom: 1.0,
            energy: 0.0,
            centroid: 0.5,
            target_re: re,
            target_im: im,
            rng_state: 0xADE1B207CAFE,
            target_index: 0,
            palette_index: 0,
            joints: Self::init_ragdoll(5.0, -10.0),
            drift_x: 0.0,
            drift_y: 0.0,
            tumble: 0.0,
        }
    }

    fn init_ragdoll(cx: f32, cy: f32) -> Vec<RagPoint> {
        // T-pose centered at (cx, cy)
        let positions = [
            (0.0, -1.8),   // head
            (0.0, -1.3),   // neck
            (-0.6, -1.0),  // l_elbow
            (-1.2, -0.8),  // l_hand
            (0.6, -1.0),   // r_elbow
            (1.2, -0.8),   // r_hand
            (0.0, -0.3),   // hip
            (-0.4, 0.4),   // l_knee
            (-0.3, 1.1),   // l_foot
            (0.4, 0.4),    // r_knee
            (0.3, 1.1),    // r_foot
        ];
        positions.iter().map(|&(dx, dy)| {
            let x = cx + dx;
            let y = cy + dy;
            RagPoint { x, y, ox: x + 0.01, oy: y - 0.02 } // tiny initial velocity
        }).collect()
    }

    fn update_ragdoll(&mut self, dt: f32) {
        // Verlet integration with gentle gravity + drift
        self.tumble += dt * 0.7;
        self.drift_x = (self.time * 0.3).sin() * 8.0 + (self.time * 0.17).cos() * 4.0;
        self.drift_y = (self.time * 0.23).cos() * 6.0 + (self.time * 0.11).sin() * 3.0;

        let gravity_y = 0.1 * dt * dt;

        // Only the hip (joint 6) is pulled toward the drift target.
        // The rest of the body flops naturally from the constraints.
        let target_x = 5.0 + self.drift_x;
        let target_y = -10.0 + self.drift_y;
        let hip = 6;

        for i in 0..self.joints.len() {
            let p = &mut self.joints[i];
            let vx = p.x - p.ox;
            let vy = p.y - p.oy;
            p.ox = p.x;
            p.oy = p.y;
            p.x += vx * 0.96; // heavy damping
            p.y += vy * 0.96 + gravity_y;

            // Hip pulled toward target — body follows
            if i == hip {
                p.x += (target_x - p.x) * 0.02;
                p.y += (target_y - p.y) * 0.02;
            }
        }

        // Constraint solver — enforce bone lengths + angle constraints
        for _ in 0..8 {
            // Distance constraints
            for &(a, b, len) in &BONES {
                let dx = self.joints[b].x - self.joints[a].x;
                let dy = self.joints[b].y - self.joints[a].y;
                let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                let diff = (len - dist) / dist * 0.5;
                let ox = dx * diff;
                let oy = dy * diff;
                self.joints[a].x -= ox;
                self.joints[a].y -= oy;
                self.joints[b].x += ox;
                self.joints[b].y += oy;
            }

            // Angle constraints — gentle position correction, matching old pos to avoid energy
            // (parent, joint, child, min_cos) — neck gets tighter constraint
            let angle_triples: [(usize, usize, usize, f32); 7] = [
                (0, 1, 6, -1.0),
                (1, 2, 3, -0.2),
                (1, 4, 5, -0.2),
                (1, 6, 7, -0.2),
                (1, 6, 9, -0.2),
                (6, 7, 8, -0.2),
                (6, 9, 10, -0.2),
            ];

            for &(a, b, c, min_cos) in &angle_triples {
                let bax = self.joints[a].x - self.joints[b].x;
                let bay = self.joints[a].y - self.joints[b].y;
                let bcx = self.joints[c].x - self.joints[b].x;
                let bcy = self.joints[c].y - self.joints[b].y;
                let la = (bax * bax + bay * bay).sqrt().max(0.001);
                let lc = (bcx * bcx + bcy * bcy).sqrt().max(0.001);
                let dot = (bax * bcx + bay * bcy) / (la * lc);
                if dot > min_cos {
                    let push = (dot - min_cos) * 0.0375;
                    let perp_x = -bay / la;
                    let perp_y = bax / la;
                    let side = bcx * perp_x + bcy * perp_y;
                    let sign = if side >= 0.0 { 1.0 } else { -1.0 };
                    let dx = perp_x * push * sign;
                    let dy = perp_y * push * sign;
                    // Move position and old position together — no velocity injection
                    self.joints[c].x += dx;
                    self.joints[c].y += dy;
                    self.joints[c].ox += dx;
                    self.joints[c].oy += dy;
                }
            }
        }

    }

    /// Get GPU uniform data for the renderer's Mandelbrot pass.
    pub fn gpu_uniforms(&self, aspect: f32) -> MandelbrotUniforms {
        let (num_colors, palette) = PALETTES[self.palette_index];
        MandelbrotUniforms {
            center_re: self.target_re,
            center_im: self.target_im,
            zoom: self.zoom,
            time: self.time,
            max_iter: 128 + (self.energy * 128.0) as u32,
            color_offset: self.time * 0.1 + self.centroid,
            aspect,
            num_colors,
            palette,
        }
    }

    /// Whether GPU rendering is available (always true — CPU fallback kept for reference)
    pub fn use_gpu(&self) -> bool { true }
}

impl AudioEffect for Mandelbrot {
    fn pass(&self) -> RenderPass { RenderPass::Transparent }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;
        let target_energy = audio.bands.iter().sum::<f32>() / 7.0;
        self.energy += (target_energy - self.energy) * 3.0 * audio.dt;
        self.centroid += (audio.centroid - self.centroid) * 2.0 * audio.dt;

        // Zoom speed — base rate + audio boost
        let zoom_speed = 0.3 + self.energy * 0.5;
        self.zoom *= 1.0 + zoom_speed * audio.dt;

        // Reset zoom at f32 precision limit — pick a new random target
        if self.zoom > 1e6 {
            self.zoom = 1.0;
            self.target_index = (self.target_index + 1) % ZOOM_TARGETS.len();
            self.palette_index = (self.palette_index + 1) % PALETTES.len();
            let (re, im) = ZOOM_TARGETS[self.target_index];
            self.target_re = re;
            self.target_im = im;
            self.joints = Self::init_ragdoll(5.0, -10.0);
        }

        self.update_ragdoll(audio.dt);
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        // Stick figure rendered as transparent geometry on top of GPU fractal background.
        // Scale shrinks as zoom increases — figure "falls away" into the fractal.
        let scale = 1.0 / (1.0 + self.zoom.log2() * 0.4);
        let alpha = scale.clamp(0.1, 1.0);
        let color = [1.0f32, 1.0, 1.0, alpha * 0.8];
        let thickness = 0.08 * scale;
        let head_size = 0.4 * scale;
        let z = 0.5; // in front of board
        let normal = [0.0f32, 0.0, 1.0];

        // Draw bones as camera-facing line quads
        for &(a, b, _) in &BONES {
            let p0 = &self.joints[a];
            let p1 = &self.joints[b];
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = -dy / len * thickness;
            let ny = dx / len * thickness;

            let base = verts.len() as u32;
            verts.push(Vertex { position: [p0.x + nx, p0.y + ny, z], normal, color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p0.x - nx, p0.y - ny, z], normal, color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p1.x - nx, p1.y - ny, z], normal, color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p1.x + nx, p1.y + ny, z], normal, color, uv: [0.0, 0.0] });
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }

        // Rounded caps at every joint — soft circles at line thickness size
        let cap = thickness * 1.2;
        for p in &self.joints {
            let base = verts.len() as u32;
            verts.push(Vertex { position: [p.x - cap, p.y - cap, z], normal, color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [p.x + cap, p.y - cap, z], normal, color, uv: [ 1.0, -1.0] });
            verts.push(Vertex { position: [p.x + cap, p.y + cap, z], normal, color, uv: [ 1.0,  1.0] });
            verts.push(Vertex { position: [p.x - cap, p.y + cap, z], normal, color, uv: [-1.0,  1.0] });
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }

        // Head — larger soft circle
        let head = &self.joints[0];
        let s = head_size;
        let base = verts.len() as u32;
        verts.push(Vertex { position: [head.x - s, head.y - s, z], normal, color, uv: [-1.0, -1.0] });
        verts.push(Vertex { position: [head.x + s, head.y - s, z], normal, color, uv: [ 1.0, -1.0] });
        verts.push(Vertex { position: [head.x + s, head.y + s, z], normal, color, uv: [ 1.0,  1.0] });
        verts.push(Vertex { position: [head.x - s, head.y + s, z], normal, color, uv: [-1.0,  1.0] });
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
    }
}
