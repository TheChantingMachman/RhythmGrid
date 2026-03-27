// Fluid effect — momentum-based particles driven by curl noise forces.
// Unlike flow_field (which sets velocity directly from noise), fluid particles
// have inertia: curl noise applies acceleration, drag is minimal, so particles
// glide and drift through the field with realistic fluid-like motion.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::{Vertex, push_cube_3d};

/// 3D value noise
fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let fz = z - z.floor();
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let uz = fz * fz * (3.0 - 2.0 * fz);
    let hash = |px: i32, py: i32, pz: i32| -> f32 {
        let h = (px as u32).wrapping_mul(374761393)
            .wrapping_add((py as u32).wrapping_mul(668265263))
            .wrapping_add((pz as u32).wrapping_mul(1274126177));
        let h = h ^ (h >> 13);
        let h = h.wrapping_mul(1103515245);
        (h as f32) / (u32::MAX as f32)
    };
    let ix1 = ix.wrapping_add(1);
    let iy1 = iy.wrapping_add(1);
    let iz1 = iz.wrapping_add(1);
    let n000 = hash(ix, iy, iz);  let n100 = hash(ix1, iy, iz);
    let n010 = hash(ix, iy1, iz); let n110 = hash(ix1, iy1, iz);
    let n001 = hash(ix, iy, iz1); let n101 = hash(ix1, iy, iz1);
    let n011 = hash(ix, iy1, iz1);let n111 = hash(ix1, iy1, iz1);
    let nx00 = n000 + ux * (n100 - n000);
    let nx10 = n010 + ux * (n110 - n010);
    let nx01 = n001 + ux * (n101 - n001);
    let nx11 = n011 + ux * (n111 - n011);
    let nxy0 = nx00 + uy * (nx10 - nx00);
    let nxy1 = nx01 + uy * (nx11 - nx01);
    nxy0 + uz * (nxy1 - nxy0)
}

/// 3D curl noise — divergence-free force field
fn curl3d(x: f32, y: f32, z: f32, scale: f32, time: f32) -> (f32, f32, f32) {
    let eps = 0.01;
    let sx = x * scale + time * 0.08;
    let sy = y * scale + time * 0.06;
    let sz = z * scale + time * 0.04;
    let n1 = |x, y, z| noise3d(x, y, z);
    let n2 = |x, y, z| noise3d(x + 31.4, y + 47.9, z + 12.7);
    let n3 = |x, y, z| noise3d(x + 71.2, y + 13.6, z + 93.1);
    let dn3_dy = (n3(sx, sy+eps, sz) - n3(sx, sy-eps, sz)) / (2.0 * eps);
    let dn2_dz = (n2(sx, sy, sz+eps) - n2(sx, sy, sz-eps)) / (2.0 * eps);
    let dn1_dz = (n1(sx, sy, sz+eps) - n1(sx, sy, sz-eps)) / (2.0 * eps);
    let dn3_dx = (n3(sx+eps, sy, sz) - n3(sx-eps, sy, sz)) / (2.0 * eps);
    let dn2_dx = (n2(sx+eps, sy, sz) - n2(sx-eps, sy, sz)) / (2.0 * eps);
    let dn1_dy = (n1(sx, sy+eps, sz) - n1(sx, sy-eps, sz)) / (2.0 * eps);
    (dn3_dy - dn2_dz, dn1_dz - dn3_dx, dn2_dx - dn1_dy)
}

struct Particle {
    x: f32, y: f32, z: f32,
    vx: f32, vy: f32, vz: f32,  // velocity — persists between frames
    life: f32,
    max_life: f32,
    hue: f32,
    size: f32,
}

pub struct Fluid {
    particles: Vec<Particle>,
    time: f32,
    rng_state: u64,
    turbulence: f32,
    energy: f32,
    centroid: f32,
    spawn_accum: f32,
    // Tumbling pieces — tetrominos that fly through the field, creating turbulence
    tumble_timer: f32,
    tumbles: Vec<TumblePiece>,
}

const SPAWN_X_MIN: f32 = -12.0;
const SPAWN_X_MAX: f32 = 22.0;
const SPAWN_Y_MIN: f32 = -25.0;
const SPAWN_Y_MAX: f32 = 5.0;
const SPAWN_Z_MIN: f32 = -4.0;
const SPAWN_Z_MAX: f32 = -0.5;
const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

// Physics tuning
const AMBIENT_FORCE: f32 = 0.05;   // near-zero ambient curl — particles barely drift
const TUMBLE_FORCE: f32 = 2.0;     // curl force radiating from tumbling piece
const TUMBLE_RADIUS: f32 = 5.0;   // influence radius around each cell
const TUMBLE_DURATION: f32 = 18.0; // seconds for piece to cross the scene
const TUMBLE_INTERVAL: f32 = 3.0; // seconds between tumbling pieces

impl TumblePiece {
    fn cell_positions(&self) -> Vec<(f32, f32, f32)> {
        let cells = &PIECE_CELLS[self.piece % 7];
        let (sx, cx) = (self.ax.sin(), self.ax.cos());
        let (sy, cy) = (self.ay.sin(), self.ay.cos());
        let (sz, cz) = (self.az.sin(), self.az.cos());
        cells.iter().map(|&(r, c)| {
            let y1 = r * cx;
            let z1 = r * sx;
            let x2 = c * cy + z1 * sy;
            let z2 = -c * sy + z1 * cy;
            let x3 = x2 * cz - y1 * sz;
            let y3 = x2 * sz + y1 * cz;
            (x3 + self.x, y3 + self.y, z2 + self.z)
        }).collect()
    }
}

struct TumblePiece {
    life: f32,
    x: f32, y: f32, z: f32,
    vx: f32, vy: f32,
    ax: f32, ay: f32, az: f32,
    spin: [f32; 3],
    piece: usize,
}
const TUMBLE_SPEED: f32 = 6.0;    // world units per second flight speed (halved)

/// Tetromino cell offsets (row, col) for tumbling piece rendering/physics.
const PIECE_CELLS: [[(f32, f32); 4]; 7] = [
    [(0.0,-1.0), (0.0, 0.0), (0.0, 1.0), (0.0, 2.0)],  // I
    [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0)],   // O
    [(0.0,-1.0), (0.0, 0.0), (0.0, 1.0), (1.0, 0.0)],   // T
    [(0.0, 0.0), (0.0, 1.0), (1.0,-1.0), (1.0, 0.0)],   // S
    [(0.0,-1.0), (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],   // Z
    [(0.0,-1.0), (0.0, 0.0), (0.0, 1.0), (1.0,-1.0)],   // J
    [(0.0,-1.0), (0.0, 0.0), (0.0, 1.0), (1.0, 1.0)],   // L
];
const DRAG: f32 = 0.003;          // very little drag — momentum persists
const Z_DAMPEN: f32 = 0.3;

impl Fluid {
    pub fn new() -> Self {
        Fluid {
            particles: Vec::with_capacity(1000),
            time: 0.0,
            rng_state: 0xF101DCAFEB0BA,
            turbulence: 1.0,
            energy: 0.0,
            centroid: 0.5,
            spawn_accum: 0.0,
            tumble_timer: 2.0,
            tumbles: Vec::new(),
        }
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng_state >> 33) as f32) / (u32::MAX as f32 / 2.0)
    }

    fn spawn(&mut self, count: usize) {
        for _ in 0..count {
            let x = SPAWN_X_MIN + self.rand_f32().abs() * (SPAWN_X_MAX - SPAWN_X_MIN);
            let y = SPAWN_Y_MIN + self.rand_f32().abs() * (SPAWN_Y_MAX - SPAWN_Y_MIN);
            let z = SPAWN_Z_MIN + self.rand_f32().abs() * (SPAWN_Z_MAX - SPAWN_Z_MIN);
            let life = 8.0 + self.rand_f32().abs() * 12.0; // long-lived — they drift slowly
            let hue = self.rand_f32().abs(); // full 0-1 range for red/white coin flip
            let size = 0.03 + self.rand_f32().abs() * 0.05;
            self.particles.push(Particle {
                x, y, z,
                vx: 0.0, vy: 0.0, vz: 0.0, // start at rest — field accelerates them
                life, max_life: life, hue, size,
            });
        }
    }

    fn hue_to_color(hue: f32) -> [f32; 4] {
        // Randomly red or white based on hue threshold
        if hue.fract() > 0.5 {
            [1.0, 0.15, 0.1, 0.6] // red
        } else {
            [1.0, 0.95, 0.9, 0.6] // warm white
        }
    }
}

impl AudioEffect for Fluid {
    fn pass(&self) -> RenderPass { RenderPass::Transparent }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;
        let target_energy = audio.bands.iter().sum::<f32>() / 7.0;
        self.energy += (target_energy - self.energy) * 3.0 * audio.dt;
        self.centroid += (audio.centroid - self.centroid) * 2.0 * audio.dt;
        self.turbulence += (audio.flux * 3.0 + 0.5 - self.turbulence) * 2.0 * audio.dt;

        let spawn_rate = 24.0 + self.energy * 120.0;
        self.spawn_accum += spawn_rate * audio.dt;
        for band in 0..7 {
            if audio.band_beats[band] > 0.9 {
                self.spawn_accum += 20.0;
            }
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        for p in &self.particles {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let alpha = if t > 0.9 { (1.0 - t) * 10.0 } else { t.sqrt() };
            let [r, g, b, a] = Fluid::hue_to_color(p.hue);
            let color = [r, g, b, a * alpha * 0.7];

            // Soft circle billboard
            let s = p.size * 0.5;
            let base = verts.len() as u32;
            verts.push(Vertex { position: [p.x - s, p.y - s, p.z], normal: NORMAL, color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [p.x + s, p.y - s, p.z], normal: NORMAL, color, uv: [ 1.0, -1.0] });
            verts.push(Vertex { position: [p.x + s, p.y + s, p.z], normal: NORMAL, color, uv: [ 1.0,  1.0] });
            verts.push(Vertex { position: [p.x - s, p.y + s, p.z], normal: NORMAL, color, uv: [-1.0,  1.0] });
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }

        // Render all tumbling pieces as 3D cubes with rotation
        for tp in &self.tumbles {
            let fade = (tp.life / TUMBLE_DURATION).clamp(0.0, 1.0);
            let piece_alpha = fade.sqrt() * 0.5;
            let color = [0.4, 0.6, 1.0, piece_alpha];
            let cells = &PIECE_CELLS[tp.piece % 7];

            let (sx, cx_r) = (tp.ax.sin(), tp.ax.cos());
            let (sy, cy_r) = (tp.ay.sin(), tp.ay.cos());
            let (sz, cz_r) = (tp.az.sin(), tp.az.cos());

            let mut pcx = 0.0f32;
            let mut pcy = 0.0f32;
            for &(r, c) in cells {
                pcx += c + 0.5;
                pcy += r + 0.5;
            }
            pcx /= cells.len() as f32;
            pcy /= cells.len() as f32;

            let start_vert = verts.len();
            for &(r, c) in cells {
                push_cube_3d(verts, indices, c - pcx, r - pcy, 0.6, color, 0.3, 0, 0.0);
            }

            for v in &mut verts[start_vert..] {
                let p = v.position;
                let y1 = p[1] * cx_r - p[2] * sx;
                let z1 = p[1] * sx + p[2] * cx_r;
                let x2 = p[0] * cy_r + z1 * sy;
                let z2 = -p[0] * sy + z1 * cy_r;
                let x3 = x2 * cz_r - y1 * sz;
                let y3 = x2 * sz + y1 * cz_r;
                v.position = [x3 + tp.x, y3 + tp.y, z2 + tp.z];

                let n = v.normal;
                let ny1 = n[1] * cx_r - n[2] * sx;
                let nz1 = n[1] * sx + n[2] * cx_r;
                let nx2 = n[0] * cy_r + nz1 * sy;
                let nz2 = -n[0] * sy + nz1 * cy_r;
                let nx3 = nx2 * cz_r - ny1 * sz;
                let ny3 = nx2 * sz + ny1 * cz_r;
                v.normal = [nx3, ny3, nz2];
            }
        }
    }
}

/// Tick particles — mostly still, with a tumbling piece that creates curl turbulence.
pub fn tick_particles(fluid: &mut Fluid, dt: f32) {
    let to_spawn = fluid.spawn_accum as usize;
    if to_spawn > 0 {
        fluid.spawn(to_spawn.min(40));
        fluid.spawn_accum -= to_spawn as f32;
    }

    // Update existing tumbling pieces
    for tp in &mut fluid.tumbles {
        tp.x += tp.vx * dt;
        tp.y += tp.vy * dt;
        tp.ax += tp.spin[0] * dt;
        tp.ay += tp.spin[1] * dt;
        tp.az += tp.spin[2] * dt;
        tp.life -= dt;
    }
    fluid.tumbles.retain(|tp| tp.life > 0.0);

    // Spawn new tumbling piece on timer
    fluid.tumble_timer -= dt;
    if fluid.tumble_timer <= 0.0 {
        fluid.tumble_timer = TUMBLE_INTERVAL;
        let angle = fluid.rand_f32().abs() * std::f32::consts::TAU;
        let rz = fluid.rand_f32();
        let rs0 = fluid.rand_f32().abs();
        let rs1 = fluid.rand_f32().abs();
        let rs2 = fluid.rand_f32().abs();
        let rp = fluid.rand_f32().abs();
        fluid.tumbles.push(TumblePiece {
            life: TUMBLE_DURATION,
            x: 5.0 - angle.cos() * 25.0,
            y: -10.0 - angle.sin() * 25.0,
            z: -2.0 + rz * 1.0,
            vx: angle.cos() * TUMBLE_SPEED,
            vy: angle.sin() * TUMBLE_SPEED,
            ax: 0.0, ay: 0.0, az: 0.0,
            spin: [0.75 + rs0 * 1.5, 1.0 + rs1 * 2.0, 0.25 + rs2 * 0.75],
            piece: (rp * 7.0) as usize,
        });
    }

    // Collect all tumble cell positions for physics
    let tumble_cells: Vec<(f32, f32, f32)> = fluid.tumbles.iter()
        .flat_map(|tp| tp.cell_positions())
        .collect();

    let noise_scale = 0.12 * fluid.turbulence;
    let drag_factor = 1.0 - DRAG * dt.min(0.1);

    for p in &mut fluid.particles {
        // Ambient: barely-there curl noise drift
        let (fx1, fy1, fz1) = curl3d(p.x, p.y, p.z, noise_scale, fluid.time);
        let mut ax = fx1 * AMBIENT_FORCE;
        let mut ay = fy1 * AMBIENT_FORCE;
        let mut az = fz1 * AMBIENT_FORCE * Z_DAMPEN;

        // Tumbling piece: each cell radiates curl force outward
        for &(cx, cy, cz) in &tumble_cells {
            let dx = p.x - cx;
            let dy = p.y - cy;
            let dz = p.z - cz;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < TUMBLE_RADIUS && dist > 0.1 {
                let falloff = 1.0 - dist / TUMBLE_RADIUS;
                let strength = TUMBLE_FORCE * falloff * falloff;
                // Radial push outward
                let inv = strength / dist;
                ax += dx * inv * 0.5;
                ay += dy * inv * 0.5;
                az += dz * inv * 0.3;
                // Curl swirl around the cell
                let (swx, swy, swz) = curl3d(p.x, p.y, p.z, noise_scale * 2.0, fluid.time * 1.5);
                ax += swx * strength;
                ay += swy * strength;
                az += swz * strength * Z_DAMPEN;
            }
        }

        p.vx += ax * dt;
        p.vy += ay * dt;
        p.vz += az * dt;

        p.vx *= drag_factor;
        p.vy *= drag_factor;
        p.vz *= drag_factor;

        // Soft Z boundary
        if p.z < SPAWN_Z_MIN + 0.5 { p.vz += (SPAWN_Z_MIN + 0.5 - p.z).abs() * 2.0 * dt; }
        if p.z > SPAWN_Z_MAX - 0.2 { p.vz -= (p.z - SPAWN_Z_MAX + 0.2).abs() * 2.0 * dt; }

        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.z += p.vz * dt;
        p.life -= dt;
    }

    fluid.particles.retain(|p| p.life > 0.0);
}
