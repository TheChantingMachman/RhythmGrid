// 3D Pipes effect — seven simultaneous pipes growing through a right-angle grid.
// Each pipe represents a frequency band. The pipe's "fill" (neon glow) bounces
// with the corresponding band's audio level. The whole structure slowly rotates.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::Vertex;

const NUM_PIPES: usize = 7;
const MAX_SEGMENTS: usize = 200;  // max segments per pipe before it resets
const GROW_INTERVAL: f32 = 0.3;   // seconds between new segments
const GRID_STEP: f32 = 1.0;       // world units per grid step
const PIPE_THICKNESS: f32 = 0.09;
const JOINT_SIZE: f32 = 0.13;
const MIN_LEG_LENGTH: usize = 3; // minimum segments before allowing a turn

// Band colors (dark base — glow comes from fill)
const BAND_COLORS: [[f32; 3]; 7] = [
    [0.15, 0.15, 0.25], // sub-bass — dark blue-grey
    [0.15, 0.15, 0.25], // bass
    [0.15, 0.15, 0.25], // low-mids
    [0.15, 0.15, 0.25], // mids
    [0.15, 0.15, 0.25], // upper-mids
    [0.15, 0.15, 0.25], // presence
    [0.15, 0.15, 0.25], // brilliance
];

// Glow colors when filled (neon)
const GLOW_COLORS: [[f32; 3]; 7] = [
    [0.3, 0.3, 0.8],  // sub-bass — blue
    [0.3, 0.5, 0.8],  // bass — cyan-blue
    [0.3, 0.7, 0.5],  // low-mids — teal
    [0.5, 0.8, 0.3],  // mids — green
    [0.8, 0.8, 0.3],  // upper-mids — yellow
    [0.8, 0.5, 0.3],  // presence — orange
    [0.8, 0.3, 0.3],  // brilliance — red
];

#[derive(Clone, Copy)]
struct Segment {
    x: f32, y: f32, z: f32,         // start position
    dir: usize,                      // direction index (0=+x, 1=-x, 2=+y, 3=-y, 4=+z, 5=-z)
}

const DIRECTIONS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0], [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0], [0.0, 0.0, -1.0],
];

struct Pipe {
    segments: Vec<Segment>,
    x: f32, y: f32, z: f32,
    dir: usize,
    leg_count: usize,          // segments since last turn
}

pub struct Pipes {
    pipes: Vec<Pipe>,
    time: f32,
    grow_timer: f32,
    rotation: [f32; 3],       // slow global rotation
    rng_state: u64,
    band_levels: [f32; 7],    // smoothed audio levels for fill
}

impl Pipes {
    pub fn new() -> Self {
        let rng_state: u64 = 0xA1BE5CAFE;
        let mut pipes = Vec::new();
        for i in 0..NUM_PIPES {
            // Spread starting positions so pipes don't overlap
            let offset = (i as f32 - 3.0) * 0.8;
            pipes.push(Pipe {
                segments: Vec::new(),
                x: offset,
                y: 0.0,
                z: 0.0,
                dir: (i * 2) % 6,
                leg_count: 0,
            });
        }
        Pipes {
            pipes,
            time: 0.0,
            grow_timer: 0.0,
            rotation: [0.0; 3],
            rng_state,
            band_levels: [0.0; 7],
        }
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng_state >> 33) as f32) / (u32::MAX as f32 / 2.0)
    }

    fn rotate_point(p: [f32; 3], ax: f32, ay: f32, az: f32) -> [f32; 3] {
        let (sx, cx) = (ax.sin(), ax.cos());
        let (sy, cy) = (ay.sin(), ay.cos());
        let (sz, cz) = (az.sin(), az.cos());
        let y1 = p[1] * cx - p[2] * sx;
        let z1 = p[1] * sx + p[2] * cx;
        let x2 = p[0] * cy + z1 * sy;
        let z2 = -p[0] * sy + z1 * cy;
        let x3 = x2 * cz - y1 * sz;
        let y3 = x2 * sz + y1 * cz;
        [x3, y3, z2]
    }

    fn grow_pipes(&mut self) {
        for i in 0..self.pipes.len() {
            let pipe = &mut self.pipes[i];
            pipe.segments.push(Segment {
                x: pipe.x, y: pipe.y, z: pipe.z,
                dir: pipe.dir,
            });

            let d = DIRECTIONS[pipe.dir];
            pipe.x += d[0] * GRID_STEP;
            pipe.y += d[1] * GRID_STEP;
            pipe.z += d[2] * GRID_STEP;
            pipe.leg_count += 1;

            if pipe.segments.len() >= MAX_SEGMENTS {
                pipe.segments.clear();
                pipe.leg_count = 0;
                let offset = (i as f32 - 3.0) * 0.8;
                pipe.x = offset;
                pipe.y = 0.0;
                pipe.z = 0.0;
            }
        }

        // Random right-angle turns — only if minimum leg length met
        let num = self.pipes.len();
        let mut turns: Vec<Option<usize>> = Vec::new();
        for i in 0..num {
            if self.pipes[i].leg_count < MIN_LEG_LENGTH {
                turns.push(None);
                continue;
            }
            let should_turn = self.rand_f32().abs() > 0.4;
            if should_turn {
                let current = self.pipes[i].dir;
                let mut new_dir;
                loop {
                    new_dir = (self.rand_f32().abs() * 6.0) as usize % 6;
                    if new_dir / 2 != current / 2 { break; }
                }
                turns.push(Some(new_dir));
            } else {
                turns.push(None);
            }
        }
        for (i, turn) in turns.into_iter().enumerate() {
            if let Some(dir) = turn {
                self.pipes[i].dir = dir;
                self.pipes[i].leg_count = 0;
            }
        }
    }
}

impl AudioEffect for Pipes {
    fn pass(&self) -> RenderPass { RenderPass::Transparent }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;

        // Slow rotation
        self.rotation[0] += audio.dt * 0.05;
        self.rotation[1] += audio.dt * 0.08;
        self.rotation[2] += audio.dt * 0.03;

        // Smooth band levels
        for i in 0..7 {
            let target = audio.bands_norm[i];
            self.band_levels[i] += (target - self.band_levels[i]) * 5.0 * audio.dt;
        }

        // Grow pipes periodically
        self.grow_timer += audio.dt;
        if self.grow_timer >= GROW_INTERVAL {
            self.grow_timer -= GROW_INTERVAL;
            self.grow_pipes();
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        let center = [5.0f32, -10.0, -2.0]; // world-space center
        let cam_dir = [0.0f32, 0.0, 1.0];
        let normal = [0.0f32, 0.0, 1.0];

        for (pi, pipe) in self.pipes.iter().enumerate() {
            let band = pi % 7;
            let level = self.band_levels[band];
            let base_col = BAND_COLORS[band];
            let glow_col = GLOW_COLORS[band];

            let num_segs = pipe.segments.len();
            if num_segs == 0 { continue; }

            // Fill point — segments below this index are "filled" (glowing)
            let fill_segs = (level * num_segs as f32) as usize;

            for (si, seg) in pipe.segments.iter().enumerate() {
                let d = DIRECTIONS[seg.dir];
                let end = [
                    seg.x + d[0] * GRID_STEP,
                    seg.y + d[1] * GRID_STEP,
                    seg.z + d[2] * GRID_STEP,
                ];

                // Rotate into world space
                let p0 = Self::rotate_point([seg.x, seg.y, seg.z], self.rotation[0], self.rotation[1], self.rotation[2]);
                let p1 = Self::rotate_point(end, self.rotation[0], self.rotation[1], self.rotation[2]);
                let p0 = [p0[0] + center[0], p0[1] + center[1], p0[2] + center[2]];
                let p1 = [p1[0] + center[0], p1[1] + center[1], p1[2] + center[2]];

                // Color: filled segments glow, unfilled are dark
                let is_filled = si < fill_segs;
                let (r, g, b, a) = if is_filled {
                    let glow_strength = 0.5 + level * 0.5;
                    (glow_col[0] * glow_strength, glow_col[1] * glow_strength, glow_col[2] * glow_strength, 0.6)
                } else {
                    (base_col[0], base_col[1], base_col[2], 0.15)
                };
                let color = [r, g, b, a];

                // Camera-facing quad for the pipe segment
                let dx = p1[0] - p0[0];
                let dy = p1[1] - p0[1];
                let dz = p1[2] - p0[2];
                let cross = [
                    dy * cam_dir[2] - dz * cam_dir[1],
                    dz * cam_dir[0] - dx * cam_dir[2],
                    dx * cam_dir[1] - dy * cam_dir[0],
                ];
                let clen = (cross[0]*cross[0] + cross[1]*cross[1] + cross[2]*cross[2]).sqrt().max(0.001);
                let nx = cross[0] / clen * PIPE_THICKNESS;
                let ny = cross[1] / clen * PIPE_THICKNESS;
                let nz = cross[2] / clen * PIPE_THICKNESS;

                let base = verts.len() as u32;
                verts.push(Vertex { position: [p0[0]+nx, p0[1]+ny, p0[2]+nz], normal, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [p0[0]-nx, p0[1]-ny, p0[2]-nz], normal, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [p1[0]-nx, p1[1]-ny, p1[2]-nz], normal, color, uv: [0.0, 0.0] });
                verts.push(Vertex { position: [p1[0]+nx, p1[1]+ny, p1[2]+nz], normal, color, uv: [0.0, 0.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

                // Joint ball at the start of each segment (soft circle)
                if si == 0 || (si > 0 && pipe.segments[si-1].dir != seg.dir) {
                    let s = JOINT_SIZE;
                    let joint_color = if is_filled {
                        [glow_col[0], glow_col[1], glow_col[2], 0.7]
                    } else {
                        [base_col[0] * 1.5, base_col[1] * 1.5, base_col[2] * 1.5, 0.2]
                    };
                    let base = verts.len() as u32;
                    verts.push(Vertex { position: [p0[0]-s, p0[1]-s, p0[2]], normal, color: joint_color, uv: [-1.0, -1.0] });
                    verts.push(Vertex { position: [p0[0]+s, p0[1]-s, p0[2]], normal, color: joint_color, uv: [ 1.0, -1.0] });
                    verts.push(Vertex { position: [p0[0]+s, p0[1]+s, p0[2]], normal, color: joint_color, uv: [ 1.0,  1.0] });
                    verts.push(Vertex { position: [p0[0]-s, p0[1]+s, p0[2]], normal, color: joint_color, uv: [-1.0,  1.0] });
                    indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
                }
            }
        }
    }
}
