// Flow field effect — 3D curl-noise-driven particles that create fluid-like motion.
// Audio-reactive: beat energy spawns particles, spectral centroid shifts
// the noise field, flux modulates turbulence. Pieces repel nearby particles,
// hard drops create expanding shockwaves.
// Operates in world space (board is x: 0..10, y: 0..-20, z: 0).

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::Vertex;

/// 3D value noise — hash-based, no dependencies.
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
    let n000 = hash(ix, iy, iz);
    let n100 = hash(ix1, iy, iz);
    let n010 = hash(ix, iy1, iz);
    let n110 = hash(ix1, iy1, iz);
    let n001 = hash(ix, iy, iz1);
    let n101 = hash(ix1, iy, iz1);
    let n011 = hash(ix, iy1, iz1);
    let n111 = hash(ix1, iy1, iz1);

    let nx00 = n000 + ux * (n100 - n000);
    let nx10 = n010 + ux * (n110 - n010);
    let nx01 = n001 + ux * (n101 - n001);
    let nx11 = n011 + ux * (n111 - n011);

    let nxy0 = nx00 + uy * (nx10 - nx00);
    let nxy1 = nx01 + uy * (nx11 - nx01);

    nxy0 + uz * (nxy1 - nxy0)
}

/// 3D curl noise — curl of three scalar noise fields gives divergence-free velocity.
fn curl_noise_3d(x: f32, y: f32, z: f32, scale: f32, time: f32) -> (f32, f32, f32) {
    let eps = 0.01;
    let sx = x * scale + time * 0.1;
    let sy = y * scale + time * 0.07;
    let sz = z * scale + time * 0.05;

    // Use three offset noise fields (offset by large constants to decorrelate)
    let n1 = |x, y, z| noise3d(x, y, z);
    let n2 = |x, y, z| noise3d(x + 31.416, y + 47.853, z + 12.734);
    let n3 = |x, y, z| noise3d(x + 71.235, y + 13.579, z + 93.147);

    // Curl = cross product of gradients:
    // vx = dN3/dy - dN2/dz
    // vy = dN1/dz - dN3/dx
    // vz = dN2/dx - dN1/dy
    let dn3_dy = (n3(sx, sy+eps, sz) - n3(sx, sy-eps, sz)) / (2.0 * eps);
    let dn2_dz = (n2(sx, sy, sz+eps) - n2(sx, sy, sz-eps)) / (2.0 * eps);
    let dn1_dz = (n1(sx, sy, sz+eps) - n1(sx, sy, sz-eps)) / (2.0 * eps);
    let dn3_dx = (n3(sx+eps, sy, sz) - n3(sx-eps, sy, sz)) / (2.0 * eps);
    let dn2_dx = (n2(sx+eps, sy, sz) - n2(sx-eps, sy, sz)) / (2.0 * eps);
    let dn1_dy = (n1(sx, sy+eps, sz) - n1(sx, sy-eps, sz)) / (2.0 * eps);

    (dn3_dy - dn2_dz, dn1_dz - dn3_dx, dn2_dx - dn1_dy)
}

struct FlowParticle {
    x: f32,
    y: f32,
    z: f32,
    life: f32,
    max_life: f32,
    hue: f32,
    size: f32,
    trail_x: f32,
    trail_y: f32,
    trail_z: f32,
}

struct Disturbance {
    x: f32,
    y: f32,
    strength: f32,
    radius: f32,
    life: f32,
}

pub struct FlowField {
    particles: Vec<FlowParticle>,
    time: f32,
    rng_state: u64,
    turbulence: f32,
    energy: f32,
    centroid: f32,
    spawn_accum: f32,
    disturbances: Vec<Disturbance>,
    piece_cells: Vec<(f32, f32)>,
}

// World-space bounds (board: x 0..10, y 0..-20)
const SPAWN_X_MIN: f32 = -12.0;
const SPAWN_X_MAX: f32 = 22.0;
const SPAWN_Y_MIN: f32 = -25.0;
const SPAWN_Y_MAX: f32 = 5.0;
const SPAWN_Z_MIN: f32 = -4.0;
const SPAWN_Z_MAX: f32 = -0.5;
const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

impl FlowField {
    pub fn new() -> Self {
        FlowField {
            particles: Vec::with_capacity(800),
            time: 0.0,
            rng_state: 0xF10EF1E1DCAFE,
            turbulence: 1.0,
            energy: 0.0,
            centroid: 0.5,
            spawn_accum: 0.0,
            disturbances: Vec::new(),
            piece_cells: Vec::new(),
        }
    }

    pub fn set_piece_cells(&mut self, cells: Vec<(f32, f32)>) {
        self.piece_cells = cells;
    }

    pub fn trigger_drop(&mut self, x: f32, y: f32, intensity: f32) {
        self.disturbances.push(Disturbance {
            x, y,
            strength: 8.0 * intensity,
            radius: 0.5,
            life: 1.2,
        });
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
            let life = 6.0 + self.rand_f32().abs() * 8.0;
            let hue = self.centroid + self.rand_f32() * 0.15;
            let size = 0.08 + self.rand_f32().abs() * 0.12;
            self.particles.push(FlowParticle {
                x, y, z, life, max_life: life, hue, size,
                trail_x: x, trail_y: y, trail_z: z,
            });
        }
    }

    fn hue_to_color(hue: f32) -> [f32; 4] {
        let h = hue.fract();
        let r = (0.1 + 0.5 * (h * 3.0).sin().abs()).min(0.8);
        let g = (0.15 + 0.4 * ((h * 3.0 + 1.0).sin().abs())).min(0.7);
        let b = (0.3 + 0.5 * ((h * 2.0 + 0.5).sin().abs())).min(1.0);
        [r, g, b, 0.6]
    }
}

impl AudioEffect for FlowField {
    fn pass(&self) -> RenderPass {
        RenderPass::Transparent
    }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;

        let target_energy = audio.bands.iter().sum::<f32>() / 7.0;
        self.energy += (target_energy - self.energy) * 3.0 * audio.dt;
        self.centroid += (audio.centroid - self.centroid) * 2.0 * audio.dt;
        self.turbulence += (audio.flux * 3.0 + 0.5 - self.turbulence) * 2.0 * audio.dt;

        let spawn_rate = 16.0 + self.energy * 80.0;
        self.spawn_accum += spawn_rate * audio.dt;

        for band in 0..7 {
            if audio.band_beats[band] > 0.9 {
                self.spawn_accum += 30.0;
            }
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        // Billboard orientation: camera looks along -Z, so right=(1,0,0), up=(0,1,0).
        // For particles at different Z depths, this gives natural parallax from the
        // perspective projection without needing per-particle camera math.
        for p in &self.particles {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let alpha = if t > 0.9 { (1.0 - t) * 10.0 } else { t.sqrt() };
            let [r, g, b, a] = FlowField::hue_to_color(p.hue);
            let color = [r, g, b, a * alpha * 0.7];

            let dx = p.x - p.trail_x;
            let dy = p.y - p.trail_y;
            let dz = p.z - p.trail_z;
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            let streak_len = len.min(p.size * 6.0);

            if streak_len > 0.01 {
                // Velocity-aligned streak: compute a perpendicular vector in the XY plane
                let xy_len = (dx * dx + dy * dy).sqrt().max(0.001);
                let nx = -dy / xy_len * p.size * 0.4;
                let ny = dx / xy_len * p.size * 0.4;
                // Back direction along velocity
                let bx = -dx / len * streak_len;
                let by = -dy / len * streak_len;
                let bz = -dz / len * streak_len;

                let tail_color = [r, g, b, a * alpha * 0.15];

                let base = verts.len() as u32;
                verts.push(Vertex { position: [p.x + nx, p.y + ny, p.z], normal: NORMAL, color, uv: [0.0, -0.5] });
                verts.push(Vertex { position: [p.x - nx, p.y - ny, p.z], normal: NORMAL, color, uv: [0.0,  0.5] });
                verts.push(Vertex { position: [p.x + bx - nx*0.5, p.y + by - ny*0.5, p.z + bz], normal: NORMAL, color: tail_color, uv: [0.9, -0.3] });
                verts.push(Vertex { position: [p.x + bx + nx*0.5, p.y + by + ny*0.5, p.z + bz], normal: NORMAL, color: tail_color, uv: [0.9,  0.3] });
                indices.extend_from_slice(&[base, base+1, base+2, base+1, base+3, base+2]);
            } else {
                // Soft circle billboard (axis-aligned in XY, at particle's Z)
                let s = p.size * 0.5;
                let base = verts.len() as u32;
                verts.push(Vertex { position: [p.x - s, p.y - s, p.z], normal: NORMAL, color, uv: [-1.0, -1.0] });
                verts.push(Vertex { position: [p.x + s, p.y - s, p.z], normal: NORMAL, color, uv: [ 1.0, -1.0] });
                verts.push(Vertex { position: [p.x + s, p.y + s, p.z], normal: NORMAL, color, uv: [ 1.0,  1.0] });
                verts.push(Vertex { position: [p.x - s, p.y + s, p.z], normal: NORMAL, color, uv: [-1.0,  1.0] });
                indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
            }
        }
    }
}

/// Tick particles — call from world.rs after update().
pub fn tick_particles(field: &mut FlowField, dt: f32) {
    let to_spawn = field.spawn_accum as usize;
    if to_spawn > 0 {
        field.spawn(to_spawn.min(30));
        field.spawn_accum -= to_spawn as f32;
    }

    for d in &mut field.disturbances {
        d.radius += 6.0 * dt;
        d.life -= dt;
        d.strength *= 0.97;
    }
    field.disturbances.retain(|d| d.life > 0.0);

    let noise_scale = 0.15 * field.turbulence;
    let speed = 2.0 + field.energy * 6.0;

    let disturbances: Vec<(f32, f32, f32, f32)> = field.disturbances.iter()
        .map(|d| (d.x, d.y, d.strength, d.radius))
        .collect();
    let piece_cells: Vec<(f32, f32)> = field.piece_cells.clone();

    for p in &mut field.particles {
        p.trail_x = p.x;
        p.trail_y = p.y;
        p.trail_z = p.z;

        // 3D curl noise
        let (cx1, cy1, cz1) = curl_noise_3d(p.x, p.y, p.z, noise_scale, field.time);
        let (cx2, cy2, cz2) = curl_noise_3d(p.x, p.y, p.z, noise_scale * 2.3, field.time * 1.7);
        let mut vx = (cx1 + cx2 * 0.4) * speed;
        let mut vy = (cy1 + cy2 * 0.4) * speed;
        let mut vz = (cz1 + cz2 * 0.4) * speed * 0.5; // dampen Z motion

        // Soft Z boundary — push back toward spawn range
        if p.z < SPAWN_Z_MIN + 0.5 { vz += (SPAWN_Z_MIN + 0.5 - p.z) * -2.0; }
        if p.z > SPAWN_Z_MAX - 0.2 { vz += (SPAWN_Z_MAX - 0.2 - p.z) * -2.0; }

        // Active piece repulsion (XY only — piece lives at z=0)
        for &(cx, cy) in &piece_cells {
            let dx = p.x - cx;
            let dy = p.y - cy;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < 9.0 && dist_sq > 0.01 {
                let dist = dist_sq.sqrt();
                let force = 1.5 / (dist + 0.5);
                vx += dx / dist * force;
                vy += dy / dist * force;
                // Also push particles away in Z for a 3D displacement feel
                vz += -0.5 * force;
            }
        }

        // Shockwave disturbances (XY ring, with Z scatter)
        for &(dx_c, dy_c, strength, radius) in &disturbances {
            let dx = p.x - dx_c;
            let dy = p.y - dy_c;
            let dist = (dx * dx + dy * dy).sqrt();
            let ring_dist = (dist - radius).abs();
            if ring_dist < 2.0 && dist > 0.1 {
                let ring_force = strength * (1.0 - ring_dist / 2.0);
                vx += dx / dist * ring_force;
                vy += dy / dist * ring_force;
                vz += ring_force * 0.3 * if p.z < -2.0 { 1.0 } else { -1.0 };
            }
        }

        p.x += vx * dt;
        p.y += vy * dt;
        p.z += vz * dt;
        p.life -= dt;
    }

    field.particles.retain(|p| p.life > 0.0);
}
