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
    stuck_to: i32,     // -1 = free, 0..3 = stuck to tetra index
    stuck_offset: [f32; 3], // offset relative to tetra center when stuck
}

// Regular tetrahedron vertices (unit scale)
const TETRA_VERTS: [[f32; 3]; 4] = [
    [ 1.0,  1.0,  1.0],
    [ 1.0, -1.0, -1.0],
    [-1.0,  1.0, -1.0],
    [-1.0, -1.0,  1.0],
];

// 4 faces: (vertex indices, outward normal, opposite vertex index)
const TETRA_FACES: [([usize; 3], [f32; 3]); 4] = [
    ([0, 1, 2], [ 0.577,  0.577, -0.577]),  // opposite vertex: 3
    ([0, 2, 3], [-0.577,  0.577,  0.577]),   // opposite vertex: 1
    ([0, 1, 3], [ 0.577, -0.577,  0.577]),   // opposite vertex: 2
    ([1, 2, 3], [-0.577, -0.577, -0.577]),   // opposite vertex: 0
];
// For each face, which vertex of the center tetra is NOT on that face
const FACE_OPPOSITE: [usize; 4] = [3, 1, 2, 0];

/// Compute the reflected apex for an outer tetra pressed face-to-face.
/// The apex is the reflection of the center's opposite vertex through the face plane.
fn reflect_through_face(face_idx: usize) -> [f32; 3] {
    let opp = TETRA_VERTS[FACE_OPPOSITE[face_idx]];
    let (_, normal) = TETRA_FACES[face_idx];
    let face_verts = TETRA_FACES[face_idx].0;
    let v0 = TETRA_VERTS[face_verts[0]];
    // Distance from opposite vertex to face plane
    let d = (opp[0]-v0[0])*normal[0] + (opp[1]-v0[1])*normal[1] + (opp[2]-v0[2])*normal[2];
    // Reflect: P' = P - 2*d*N
    [opp[0] - 2.0*d*normal[0], opp[1] - 2.0*d*normal[1], opp[2] - 2.0*d*normal[2]]
}

/// Get the 4 vertices of the i-th tetra in the assembled configuration (unit scale).
/// Tetra 0-3: outer tetras (share 3 verts with center, apex reflected outward)
/// Tetra 4: center tetra (original TETRA_VERTS)
fn assembled_tetra_verts(i: usize) -> [[f32; 3]; 4] {
    if i >= 4 {
        return TETRA_VERTS;
    }
    let face_verts = TETRA_FACES[i].0;
    let apex = reflect_through_face(i);
    [
        TETRA_VERTS[face_verts[0]],
        TETRA_VERTS[face_verts[1]],
        TETRA_VERTS[face_verts[2]],
        apex,
    ]
}

#[derive(PartialEq, Clone, Copy)]
enum CapturePhase {
    Scatter,    // tetra tumble independently, collecting particles
    Attract,    // tetra pull toward each other
    Assembled,  // big tetrahedron tumbles, collecting more
    Release,    // disappear, free particles
    Pause,      // nothing, wait to restart
}

struct MiniTetra {
    x: f32, y: f32, z: f32,
    ax: f32, ay: f32, az: f32,
    spin: [f32; 3],
    scale: f32,
    target_x: f32, target_y: f32, target_z: f32,
    // Assembled vertex positions (world space) — set during assembled phase
    assembled_verts: Option<[[f32; 3]; 4]>,
}

impl MiniTetra {
    fn rotate_point(&self, p: [f32; 3]) -> [f32; 3] {
        let (sx, cx) = (self.ax.sin(), self.ax.cos());
        let (sy, cy) = (self.ay.sin(), self.ay.cos());
        let (sz, cz) = (self.az.sin(), self.az.cos());
        let y1 = p[1] * cx - p[2] * sx;
        let z1 = p[1] * sx + p[2] * cx;
        let x2 = p[0] * cy + z1 * sy;
        let z2 = -p[0] * sy + z1 * cy;
        let x3 = x2 * cz - y1 * sz;
        let y3 = x2 * sz + y1 * cz;
        [x3, y3, z2]
    }

    /// Test if a point is inside a tetrahedron defined by explicit world-space vertices.
    fn contains_explicit(verts: &[[f32; 3]; 4], px: f32, py: f32, pz: f32) -> bool {
        let p = [px, py, pz];
        // For each face (3 vertices), check that the point is on the same side as the 4th vertex
        let faces: [[usize; 3]; 4] = [[0,1,2],[0,2,3],[0,1,3],[1,2,3]];
        let opposite: [usize; 4] = [3, 1, 2, 0];
        for fi in 0..4 {
            let a = verts[faces[fi][0]];
            let b = verts[faces[fi][1]];
            let c = verts[faces[fi][2]];
            let d = verts[opposite[fi]]; // the vertex not on this face
            // Normal of face ABC
            let ab = [b[0]-a[0], b[1]-a[1], b[2]-a[2]];
            let ac = [c[0]-a[0], c[1]-a[1], c[2]-a[2]];
            let n = [ab[1]*ac[2]-ab[2]*ac[1], ab[2]*ac[0]-ab[0]*ac[2], ab[0]*ac[1]-ab[1]*ac[0]];
            // Dot product of (D-A) with normal — tells us which side D is on
            let d_side = (d[0]-a[0])*n[0] + (d[1]-a[1])*n[1] + (d[2]-a[2])*n[2];
            // Dot product of (P-A) with normal — must be same sign as d_side
            let p_side = (p[0]-a[0])*n[0] + (p[1]-a[1])*n[1] + (p[2]-a[2])*n[2];
            if d_side * p_side < 0.0 { return false; }
        }
        true
    }

    /// Test if a point is inside this tetrahedron (in world space, using rotation)
    fn contains(&self, px: f32, py: f32, pz: f32) -> bool {
        if let Some(ref verts) = self.assembled_verts {
            return Self::contains_explicit(verts, px, py, pz);
        }
        // Transform point into local space
        let local = [px - self.x, py - self.y, pz - self.z];
        // Inverse rotation (negate angles)
        let (sx, cx) = ((-self.ax).sin(), (-self.ax).cos());
        let (sy, cy) = ((-self.ay).sin(), (-self.ay).cos());
        let (sz, cz) = ((-self.az).sin(), (-self.az).cos());
        // Reverse order: Z, Y, X
        let x1 = local[0] * cz - local[1] * sz;
        let y1 = local[0] * sz + local[1] * cz;
        let x2 = x1 * cy + local[2] * sy;
        let z2 = -x1 * sy + local[2] * cy;
        let y3 = y1 * cx - z2 * sx;
        let z3 = y1 * sx + z2 * cx;
        let lp = [x2 / self.scale, y3 / self.scale, z3 / self.scale];

        // Check all 4 face planes — point must be on the inside of all
        for &(face_verts, normal) in &TETRA_FACES {
            let v = TETRA_VERTS[face_verts[0]]; // use a vertex ON this face
            let d = (lp[0] - v[0]) * normal[0] + (lp[1] - v[1]) * normal[1] + (lp[2] - v[2]) * normal[2];
            if d > 0.0 { return false; }
        }
        true
    }
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
    // Invisible tetrahedron capture system
    tetras: Vec<MiniTetra>,
    capture_phase: CapturePhase,
    phase_timer: f32,
    assembled_rotation: [f32; 3],
    assembled_center: [f32; 3],
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
        // 4 mini tetrahedra — will assemble into a larger one
        let tetra_scale = 2.8; // 25% smaller than 3.75
        let spread = 8.0;
        // 5 tetrahedra: center (#4) + 4 outer ones that press face-to-face.
        // Each outer tetra's target is opposite the center's face — its apex points outward.
        // Face normals of center tetra (outward):
        let face_normals = [
            [ 0.577,  0.577, -0.577],
            [-0.577,  0.577,  0.577],
            [ 0.577, -0.577,  0.577],
            [-0.577, -0.577, -0.577],
        ];
        // Distance from center to face center = inradius ≈ 0.577 for unit tetra.
        // The outer tetra's center sits at 2 * inradius along the normal from center.
        let face_offset = 1.15; // distance along normal for face-to-face touching
        let mut tetra_starts: Vec<[f32; 3]> = Vec::new();
        let mut tetra_targets: Vec<[f32; 3]> = Vec::new();
        // Outer tetras 0-3
        for i in 0..4 {
            let n = face_normals[i];
            // Scatter start: spread out
            tetra_starts.push([n[0] * spread, n[1] * spread, n[2] * spread]);
            // Assembly target: face-to-face with center
            tetra_targets.push([n[0] * tetra_scale * face_offset, n[1] * tetra_scale * face_offset, n[2] * tetra_scale * face_offset]);
        }
        // Center tetra (#4)
        tetra_starts.push([0.0, 0.0, 0.0]);
        tetra_targets.push([0.0, 0.0, 0.0]);

        let tetras = (0..5).map(|i| {
            let s = tetra_starts[i];
            let t = tetra_targets[i];
            MiniTetra {
                x: 5.0 + s[0],
                y: -10.0 + s[1],
                z: -2.0 + s[2],
                ax: i as f32 * 0.5, ay: i as f32 * 0.7, az: i as f32 * 0.3,
                spin: [0.1 + i as f32 * 0.03, 0.08 + i as f32 * 0.025, 0.06],
                scale: tetra_scale,
                target_x: 5.0 + t[0],
                target_y: -10.0 + t[1],
                target_z: -2.0 + t[2],
                assembled_verts: None,
            }
        }).collect();

        FlowField {
            particles: Vec::with_capacity(800),
            time: 0.0,
            rng_state: 0xF10EF1E1DCAFE,
            turbulence: 1.0,
            energy: 0.0,
            centroid: 0.2,
            spawn_accum: 0.0,
            disturbances: Vec::new(),
            piece_cells: Vec::new(),
            tetras,
            capture_phase: CapturePhase::Scatter,
            phase_timer: 32.0, // scatter for 32 seconds first
            assembled_rotation: [0.0; 3],
            assembled_center: [5.0, -10.0, -2.0],
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
                stuck_to: -1, stuck_offset: [0.0; 3],
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

        // Boost spawn rate to compensate for stuck particles leaving the field sparse
        let stuck_count = self.particles.iter().filter(|p| p.stuck_to >= 0).count();
        let compensation = 1.0 + stuck_count as f32 * 0.1; // 10% more per stuck particle
        let spawn_rate = (32.0 + self.energy * 160.0) * compensation;
        self.spawn_accum += spawn_rate * audio.dt;

        for band in 0..7 {
            if audio.band_beats[band] > 0.9 {
                self.spawn_accum += 60.0;
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

        if p.stuck_to >= 0 {
            let ti = p.stuck_to as usize;
            if ti < field.tetras.len() {
                let t = &field.tetras[ti];
                if field.capture_phase == CapturePhase::Assembled {
                    // During assembly, position relative to assembled center
                    // with assembled rotation (whole structure is one rigid body)
                    let ar = field.assembled_rotation;
                    let ac = field.assembled_center;
                    let o = p.stuck_offset;
                    let (sx, cx) = (ar[0].sin(), ar[0].cos());
                    let (sy, cy) = (ar[1].sin(), ar[1].cos());
                    let (sz, cz) = (ar[2].sin(), ar[2].cos());
                    let y1 = o[1] * cx - o[2] * sx;
                    let z1 = o[1] * sx + o[2] * cx;
                    let x2 = o[0] * cy + z1 * sy;
                    let z2 = -o[0] * sy + z1 * cy;
                    let x3 = x2 * cz - y1 * sz;
                    let y3 = x2 * sz + y1 * cz;
                    p.x = ac[0] + x3;
                    p.y = ac[1] + y3;
                    p.z = ac[2] + z2;
                } else {
                    // During scatter/attract, use tetra's own rotation
                    let rotated = t.rotate_point(p.stuck_offset);
                    p.x = t.x + rotated[0];
                    p.y = t.y + rotated[1];
                    p.z = t.z + rotated[2];
                }
                p.trail_x = p.x;
                p.trail_y = p.y;
                p.trail_z = p.z;
            }
        } else {
            p.x += vx * dt;
            p.y += vy * dt;
            p.z += vz * dt;
            p.life -= dt;
        }
    }

    // --- Tetrahedron capture system ---
    field.phase_timer -= dt;

    // Rotate all tetras
    for t in &mut field.tetras {
        t.ax += t.spin[0] * dt;
        t.ay += t.spin[1] * dt;
        t.az += t.spin[2] * dt;
    }

    match field.capture_phase {
        CapturePhase::Scatter => {
            // Tetras tumble independently — check for particle captures
            for pi in 0..field.particles.len() {
                if field.particles[pi].stuck_to >= 0 { continue; }
                let px = field.particles[pi].x;
                let py = field.particles[pi].y;
                let pz = field.particles[pi].z;
                for ti in 0..field.tetras.len() {
                    if field.tetras[ti].contains(px, py, pz) {
                        let t = &field.tetras[ti];
                        // Compute offset in tetra's local rotated space
                        let dx = px - t.x;
                        let dy = py - t.y;
                        let dz = pz - t.z;
                        // Store in local space (inverse rotation)
                        let (sx, cx) = ((-t.ax).sin(), (-t.ax).cos());
                        let (sy, cy) = ((-t.ay).sin(), (-t.ay).cos());
                        let (sz, cz) = ((-t.az).sin(), (-t.az).cos());
                        let x1 = dx * cz - dy * sz;
                        let y1 = dx * sz + dy * cz;
                        let x2 = x1 * cy + dz * sy;
                        let z2 = -x1 * sy + dz * cy;
                        let y3 = y1 * cx - z2 * sx;
                        let z3 = y1 * sx + z2 * cx;
                        field.particles[pi].stuck_to = ti as i32;
                        field.particles[pi].stuck_offset = [x2, y3, z3];
                        break;
                    }
                }
            }
            if field.phase_timer <= 0.0 {
                field.capture_phase = CapturePhase::Attract;
                field.phase_timer = 6.0;
            }
        }
        CapturePhase::Attract => {
            // Pull tetras toward assembly positions — accelerating
            let progress = 1.0 - (field.phase_timer / 6.0).max(0.0);
            let pull = progress * progress * 8.0; // quadratic ramp, strong pull
            for t in &mut field.tetras {
                t.x += (t.target_x - t.x) * pull * dt;
                t.y += (t.target_y - t.y) * pull * dt;
                t.z += (t.target_z - t.z) * pull * dt;
                // Gradually unify rotations toward zero (they'll sync in assembled phase)
                t.ax *= 1.0 - progress * dt * 2.0;
                t.ay *= 1.0 - progress * dt * 2.0;
                t.az *= 1.0 - progress * dt * 2.0;
            }
            if field.phase_timer <= 0.0 {
                for t in &mut field.tetras {
                    t.x = t.target_x;
                    t.y = t.target_y;
                    t.z = t.target_z;
                    t.ax = 0.0; t.ay = 0.0; t.az = 0.0; // sync rotations
                }
                // Free only outer tetra particles — center keeps its captured ones
                // This gives the "skeleton" look: dense center + sparse outer surface
                for p in &mut field.particles {
                    if p.stuck_to >= 0 && p.stuck_to < 4 {
                        p.stuck_to = -1;
                        p.trail_x = p.x;
                        p.trail_y = p.y;
                        p.trail_z = p.z;
                        p.life = 12.0;
                        p.max_life = 12.0;
                    }
                }
                // Convert center tetra particles' offsets to assembled frame
                let ac = field.assembled_center;
                for p in &mut field.particles {
                    if p.stuck_to == 4 {
                        // Recompute offset relative to assembled center (rotation is 0 at start)
                        p.stuck_offset = [p.x - ac[0], p.y - ac[1], p.z - ac[2]];
                    }
                }
                field.capture_phase = CapturePhase::Assembled;
                field.phase_timer = 50.0;
            }
        }
        CapturePhase::Assembled => {
            // All tetras rotate together as one big tetrahedron
            field.assembled_rotation[0] += 0.14 * dt;
            field.assembled_rotation[1] += 0.10 * dt;
            field.assembled_rotation[2] += 0.07 * dt;

            let c = field.assembled_center;
            let (sx, cxr) = (field.assembled_rotation[0].sin(), field.assembled_rotation[0].cos());
            let (sy, cy) = (field.assembled_rotation[1].sin(), field.assembled_rotation[1].cos());
            let (sz, cz) = (field.assembled_rotation[2].sin(), field.assembled_rotation[2].cos());
            let rot = |p: [f32; 3]| -> [f32; 3] {
                let y1 = p[1] * cxr - p[2] * sx;
                let z1 = p[1] * sx + p[2] * cxr;
                let x2 = p[0] * cy + z1 * sy;
                let z2 = -p[0] * sy + z1 * cy;
                let x3 = x2 * cz - y1 * sz;
                let y3 = x2 * sz + y1 * cz;
                [x3, y3, z2]
            };
            // Compute actual world-space vertices for each tetra in the assembled config.
            // Each tetra's 4 vertices are: assembled_tetra_verts(i) * scale, rotated, + center.
            for i in 0..field.tetras.len() {
                let local_verts = assembled_tetra_verts(i);
                let scale = field.tetras[i].scale;
                let mut world_verts = [[0.0f32; 3]; 4];
                let mut cx_sum = 0.0f32;
                let mut cy_sum = 0.0f32;
                let mut cz_sum = 0.0f32;
                for (vi, lv) in local_verts.iter().enumerate() {
                    let scaled = [lv[0] * scale, lv[1] * scale, lv[2] * scale];
                    let rotated = rot(scaled);
                    world_verts[vi] = [
                        c[0] + rotated[0],
                        c[1] + rotated[1],
                        c[2] + rotated[2],
                    ];
                    cx_sum += world_verts[vi][0];
                    cy_sum += world_verts[vi][1];
                    cz_sum += world_verts[vi][2];
                }
                // Set tetra center to centroid of its vertices
                field.tetras[i].x = cx_sum / 4.0;
                field.tetras[i].y = cy_sum / 4.0;
                field.tetras[i].z = cz_sum / 4.0;
                field.tetras[i].assembled_verts = Some(world_verts);
                // Set rotation to assembled rotation for stuck particle positioning
                field.tetras[i].ax = field.assembled_rotation[0];
                field.tetras[i].ay = field.assembled_rotation[1];
                field.tetras[i].az = field.assembled_rotation[2];
            }

            // Continue capturing particles — store offset relative to assembled center
            let ar = field.assembled_rotation;
            let ac = field.assembled_center;
            let (isx, icx) = ((-ar[0]).sin(), (-ar[0]).cos());
            let (isy, icy) = ((-ar[1]).sin(), (-ar[1]).cos());
            let (isz, icz) = ((-ar[2]).sin(), (-ar[2]).cos());
            for pi in 0..field.particles.len() {
                if field.particles[pi].stuck_to >= 0 { continue; }
                let px = field.particles[pi].x;
                let py = field.particles[pi].y;
                let pz = field.particles[pi].z;
                for ti in 0..field.tetras.len() {
                    if field.tetras[ti].contains(px, py, pz) {
                        // Store offset in assembled rotation's inverse frame
                        let dx = px - ac[0];
                        let dy = py - ac[1];
                        let dz = pz - ac[2];
                        // Inverse assembled rotation (Z, Y, X reverse order)
                        let x1 = dx * icz - dy * isz;
                        let y1 = dx * isz + dy * icz;
                        let x2 = x1 * icy + dz * isy;
                        let z2 = -x1 * isy + dz * icy;
                        let y3 = y1 * icx - z2 * isx;
                        let z3 = y1 * isx + z2 * icx;
                        field.particles[pi].stuck_to = ti as i32;
                        field.particles[pi].stuck_offset = [x2, y3, z3];
                        break;
                    }
                }
            }

            if field.phase_timer <= 0.0 {
                field.capture_phase = CapturePhase::Release;
                field.phase_timer = 2.0;
            }
        }
        CapturePhase::Release => {
            // Clear assembled vertex cache
            for t in &mut field.tetras {
                t.assembled_verts = None;
            }
            // Free all stuck particles — zero velocity, ghost lingers
            for p in &mut field.particles {
                if p.stuck_to >= 0 {
                    p.stuck_to = -1;
                    p.trail_x = p.x;
                    p.trail_y = p.y;
                    p.trail_z = p.z;
                    p.life = 6.0;
                    p.max_life = 6.0;
                }
            }
            if field.phase_timer <= 0.0 {
                field.capture_phase = CapturePhase::Pause;
                field.phase_timer = 10.0;
            }
        }
        CapturePhase::Pause => {
            if field.phase_timer <= 0.0 {
                // Reset tetras to scattered positions
                let spread = 8.0;
                let face_normals = [
                    [ 0.577f32,  0.577, -0.577],
                    [-0.577,  0.577,  0.577],
                    [ 0.577, -0.577,  0.577],
                    [-0.577, -0.577, -0.577],
                ];
                for (i, t) in field.tetras.iter_mut().enumerate() {
                    let v = if i < 4 { face_normals[i] } else { [0.0, 0.0, 0.0] };
                    t.x = 5.0 + v[0] * spread;
                    t.y = -10.0 + v[1] * spread;
                    t.z = -2.0 + v[2] * spread;
                }
                field.capture_phase = CapturePhase::Scatter;
                field.phase_timer = 32.0;
                field.assembled_rotation = [0.0; 3];
            }
        }
    }

    field.particles.retain(|p| p.life > 0.0 || p.stuck_to >= 0);
}
