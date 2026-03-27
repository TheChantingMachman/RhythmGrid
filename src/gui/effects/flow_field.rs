// Flow field effect — 3D curl-noise-driven particles that create fluid-like motion.
// Audio-reactive: beat energy spawns particles, spectral centroid shifts
// the noise field, flux modulates turbulence. Pieces repel nearby particles,
// hard drops create expanding shockwaves.
// Operates in world space (board is x: 0..10, y: 0..-20, z: 0).

use super::{AudioEffect, AudioFrame, GpuEffect, RenderContext, RenderPass};
use super::super::drawing::Vertex;
use super::super::renderer::GpuOitDrawCmd;

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
    capture_cooldown: f32,  // seconds to skip capture after transition
    // GPU compute port
    gpu: Option<FlowFieldGpu>,
    gpu_free_all_stuck: bool,
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
        let tetra_scale = 2.8;
        let spread = 8.0;
        // 5 tetrahedra: center (#4) + 4 outer ones that press face-to-face.
        // Compute assembly targets from the EXACT vertex positions (assembled_tetra_verts)
        // so attract destinations match assembled positions perfectly — no snap.
        let mut tetra_starts: Vec<[f32; 3]> = Vec::new();
        let mut tetra_targets: Vec<[f32; 3]> = Vec::new();
        let face_normals = [
            [ 0.577f32,  0.577, -0.577],
            [-0.577,  0.577,  0.577],
            [ 0.577, -0.577,  0.577],
            [-0.577, -0.577, -0.577],
        ];
        for i in 0..5 {
            let verts = assembled_tetra_verts(i);
            // Centroid of the assembled tetra's 4 vertices (at unit scale)
            let cx = (verts[0][0] + verts[1][0] + verts[2][0] + verts[3][0]) / 4.0;
            let cy = (verts[0][1] + verts[1][1] + verts[2][1] + verts[3][1]) / 4.0;
            let cz = (verts[0][2] + verts[1][2] + verts[2][2] + verts[3][2]) / 4.0;
            tetra_targets.push([cx * tetra_scale, cy * tetra_scale, cz * tetra_scale]);
            // Scatter start: spread along face normals (or center for #4)
            if i < 4 {
                let n = face_normals[i];
                tetra_starts.push([n[0] * spread, n[1] * spread, n[2] * spread]);
            } else {
                tetra_starts.push([0.0, 0.0, 0.0]);
            }
        }

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
            capture_cooldown: 0.0,
            gpu: None,
            gpu_free_all_stuck: false,
        }
    }

    /// Get GPU draw command for the OIT render pass (None if GPU inactive or no particles).
    pub fn gpu_draw_cmd(&self) -> Option<GpuOitDrawCmd<'_>> {
        let gpu = self.gpu.as_ref()?;
        if gpu.active_count == 0 { return None; }
        Some(GpuOitDrawCmd {
            pipeline: &gpu.render_pipeline,
            bind_group_1: &gpu.render_bind_group,
            instances: gpu.active_count,
        })
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
            let hue = self.centroid + self.rand_f32() * 0.4; // wider range — mix of teal and magenta
            let size = 0.04 + self.rand_f32().abs() * 0.06;
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
        let spawn_rate = (80.0 + self.energy * 400.0) * compensation;
        self.spawn_accum += spawn_rate * audio.dt;

        for band in 0..7 {
            if audio.band_beats[band] > 0.9 {
                self.spawn_accum += 150.0;
            }
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        // Billboard orientation: camera looks along -Z, so right=(1,0,0), up=(0,1,0).
        // For particles at different Z depths, this gives natural parallax from the
        // perspective projection without needing per-particle camera math.
        for p in &self.particles {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let mut alpha = if t > 0.9 { (1.0 - t) * 10.0 } else { t.sqrt() };
            // Quick fade on outer tetra particles in the last 0.5s of attract
            if self.capture_phase == CapturePhase::Attract && p.stuck_to >= 0 && p.stuck_to < 4 {
                let fade = (self.phase_timer / 0.5).clamp(0.0, 1.0);
                alpha *= fade;
            }
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

        // DEBUG: uncomment to show magenta wireframe edges for all tetrahedra
        // let debug_color = [1.0f32, 0.0, 1.0, 0.8];
        // let thickness = 0.02;
        // let cam_dir = [0.0f32, 0.0, 1.0];
        // let tetra_edges: [(usize, usize); 6] = [(0,1),(0,2),(0,3),(1,2),(1,3),(2,3)];
        // for t in &self.tetras {
        //     let world_verts: Vec<[f32; 3]> = if let Some(ref av) = t.assembled_verts {
        //         av.to_vec()
        //     } else {
        //         TETRA_VERTS.iter().map(|v| {
        //             let scaled = [v[0] * t.scale, v[1] * t.scale, v[2] * t.scale];
        //             let rotated = t.rotate_point(scaled);
        //             [rotated[0] + t.x, rotated[1] + t.y, rotated[2] + t.z]
        //         }).collect()
        //     };
        //     for &(a, b) in &tetra_edges {
        //         let p0 = world_verts[a];
        //         let p1 = world_verts[b];
        //         let dx = p1[0]-p0[0]; let dy = p1[1]-p0[1]; let dz = p1[2]-p0[2];
        //         let c = [dy*cam_dir[2]-dz*cam_dir[1], dz*cam_dir[0]-dx*cam_dir[2], dx*cam_dir[1]-dy*cam_dir[0]];
        //         let clen = (c[0]*c[0]+c[1]*c[1]+c[2]*c[2]).sqrt().max(0.001);
        //         let nx = c[0]/clen*thickness;
        //         let ny = c[1]/clen*thickness;
        //         let nz = c[2]/clen*thickness;
        //         let base = verts.len() as u32;
        //         verts.push(Vertex { position: [p0[0]+nx,p0[1]+ny,p0[2]+nz], normal: NORMAL, color: debug_color, uv: [0.0,0.0] });
        //         verts.push(Vertex { position: [p0[0]-nx,p0[1]-ny,p0[2]-nz], normal: NORMAL, color: debug_color, uv: [0.0,0.0] });
        //         verts.push(Vertex { position: [p1[0]-nx,p1[1]-ny,p1[2]-nz], normal: NORMAL, color: debug_color, uv: [0.0,0.0] });
        //         verts.push(Vertex { position: [p1[0]+nx,p1[1]+ny,p1[2]+nz], normal: NORMAL, color: debug_color, uv: [0.0,0.0] });
        //         indices.extend_from_slice(&[base,base+1,base+2,base,base+2,base+3]);
        //     }
        // }
    }
}

/// Tick particles — call from world.rs after update().
pub fn tick_particles(field: &mut FlowField, dt: f32) {
    let to_spawn = field.spawn_accum as usize;
    if to_spawn > 0 {
        let count = to_spawn.min(80);
        if field.gpu.is_some() {
            // Generate particle data using field's RNG, then push to GPU pending list
            let mut new_particles = Vec::with_capacity(count);
            for _ in 0..count {
                let x = SPAWN_X_MIN + field.rand_f32().abs() * (SPAWN_X_MAX - SPAWN_X_MIN);
                let y = SPAWN_Y_MIN + field.rand_f32().abs() * (SPAWN_Y_MAX - SPAWN_Y_MIN);
                let z = SPAWN_Z_MIN + field.rand_f32().abs() * (SPAWN_Z_MAX - SPAWN_Z_MIN);
                let life = 6.0 + field.rand_f32().abs() * 8.0;
                let hue = field.centroid + field.rand_f32() * 0.4;
                let size = 0.04 + field.rand_f32().abs() * 0.06;
                new_particles.push(ParticleGpu {
                    pos_life: [x, y, z, life],
                    trail_maxlife: [x, y, z, life],
                    attrs: [hue, size, -1.0, 0.0],
                    stuck_offset: [0.0; 4],
                });
            }
            field.gpu.as_mut().unwrap().pending_spawns.extend(new_particles);
        } else {
            field.spawn(count);
        }
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
            // Smooth lerp toward assembly positions — eases in, strong at end
            let progress = 1.0 - (field.phase_timer / 6.0).max(0.0);
            // Lerp factor: starts gentle, gets very strong so positions converge
            let lerp = (progress * progress * progress).min(0.99);
            for t in &mut field.tetras {
                t.x = t.x + (t.target_x - t.x) * lerp.max(progress * 0.3);
                t.y = t.y + (t.target_y - t.y) * lerp.max(progress * 0.3);
                t.z = t.z + (t.target_z - t.z) * lerp.max(progress * 0.3);
                // Spin slows down as they approach
                t.spin = [t.spin[0] * (1.0 - dt * progress * 3.0).max(0.0),
                          t.spin[1] * (1.0 - dt * progress * 3.0).max(0.0),
                          t.spin[2] * (1.0 - dt * progress * 3.0).max(0.0)];
            }
            if field.phase_timer <= 0.0 {
                // Smooth final placement — should already be very close
                for t in &mut field.tetras {
                    t.x = t.target_x;
                    t.y = t.target_y;
                    t.z = t.target_z;
                    t.spin = [0.0; 3]; // stop individual spinning
                }
                // Free ALL stuck particles — zero velocity, they'll drift naturally
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
                // Cooldown: don't capture for 3 seconds so freed particles drift out
                field.capture_cooldown = 0.75;
                field.capture_phase = CapturePhase::Assembled;
                field.phase_timer = 50.0;
                field.gpu_free_all_stuck = true;
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

            // Tick capture cooldown
            if field.capture_cooldown > 0.0 {
                field.capture_cooldown -= dt;
            }

            // Capture particles (only after cooldown so freed particles drift out first)
            if field.capture_cooldown <= 0.0 {
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
            } // end capture cooldown guard

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
            field.gpu_free_all_stuck = true;
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
                let spread = 5.0;
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
                    // Restore tumble spin for scatter phase
                    t.spin = [0.1 + i as f32 * 0.03, 0.08 + i as f32 * 0.025, 0.06];
                }
                field.capture_phase = CapturePhase::Scatter;
                field.phase_timer = 32.0;
                field.assembled_rotation = [0.0; 3];
            }
        }
    }

    field.particles.retain(|p| p.life > 0.0 || p.stuck_to >= 0);
}

// ===== GPU Compute Port =====

// Must exceed peak_spawn_rate × max_particle_lifetime to prevent wrapping
// from overwriting stuck particles. Peak ~2800/sec × 94s ≈ 263K. 16MB GPU buffer.
const MAX_PARTICLES: usize = 262144;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ParticleGpu {
    pos_life: [f32; 4],      // xyz = position, w = life
    trail_maxlife: [f32; 4], // xyz = trail position, w = max_life
    attrs: [f32; 4],         // x = hue, y = size, z = stuck_to (as f32, -1=free), w = pad
    stuck_offset: [f32; 4],  // xyz = offset in tetra local space, w = pad
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FlowUniformsGpu {
    time: f32,
    dt: f32,
    noise_scale: f32,
    speed: f32,
    active_count: u32,
    capture_phase: u32,
    free_all_stuck: u32,
    capture_enabled: u32,
    assembled_center: [f32; 4],
    assembled_rotation: [f32; 4],
    phase_timer: f32,
    _pad: [f32; 3],
    tetra_verts: [[f32; 4]; 20],
    tetra_centers: [[f32; 4]; 5],
    tetra_rotations: [[f32; 4]; 5],
}

const FLOW_COMPUTE_WGSL: &str = r#"
struct FlowUniforms {
    time: f32,
    dt: f32,
    noise_scale: f32,
    speed: f32,
    active_count: u32,
    capture_phase: u32,
    free_all_stuck: u32,
    capture_enabled: u32,
    assembled_center: vec4<f32>,
    assembled_rotation: vec4<f32>,
    phase_timer: f32,
    _pad0: f32, _pad1: f32, _pad2: f32,
    tetra_verts: array<vec4<f32>, 20>,
    tetra_centers: array<vec4<f32>, 5>,
    tetra_rotations: array<vec4<f32>, 5>,
};

struct Particle {
    pos_life: vec4<f32>,
    trail_maxlife: vec4<f32>,
    attrs: vec4<f32>,
    stuck_offset: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: FlowUniforms;
@group(0) @binding(1) var<storage, read_write> velocity_grid: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> particles: array<Particle>;

const GRID_X: u32 = 32u;
const GRID_Y: u32 = 32u;
const GRID_Z: u32 = 8u;
const X_MIN: f32 = -12.0;
const X_MAX: f32 = 22.0;
const Y_MIN: f32 = -25.0;
const Y_MAX: f32 = 5.0;
const Z_MIN: f32 = -4.0;
const Z_MAX: f32 = -0.5;

fn hash3d(px: i32, py: i32, pz: i32) -> f32 {
    var h = bitcast<u32>(px) * 374761393u + bitcast<u32>(py) * 668265263u + bitcast<u32>(pz) * 1274126177u;
    h = h ^ (h >> 13u);
    h = h * 1103515245u;
    return f32(h) / 4294967295.0;
}

fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    let ix = i32(floor(x));
    let iy = i32(floor(y));
    let iz = i32(floor(z));
    let fx = x - floor(x);
    let fy = y - floor(y);
    let fz = z - floor(z);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let uz = fz * fz * (3.0 - 2.0 * fz);

    let n000 = hash3d(ix, iy, iz);
    let n100 = hash3d(ix + 1, iy, iz);
    let n010 = hash3d(ix, iy + 1, iz);
    let n110 = hash3d(ix + 1, iy + 1, iz);
    let n001 = hash3d(ix, iy, iz + 1);
    let n101 = hash3d(ix + 1, iy, iz + 1);
    let n011 = hash3d(ix, iy + 1, iz + 1);
    let n111 = hash3d(ix + 1, iy + 1, iz + 1);

    let nx00 = n000 + ux * (n100 - n000);
    let nx10 = n010 + ux * (n110 - n010);
    let nx01 = n001 + ux * (n101 - n001);
    let nx11 = n011 + ux * (n111 - n011);
    let nxy0 = nx00 + uy * (nx10 - nx00);
    let nxy1 = nx01 + uy * (nx11 - nx01);
    return nxy0 + uz * (nxy1 - nxy0);
}

fn curl_noise_3d(x: f32, y: f32, z: f32, scale: f32, time: f32) -> vec3<f32> {
    let eps = 0.01;
    let sx = x * scale + time * 0.1;
    let sy = y * scale + time * 0.07;
    let sz = z * scale + time * 0.05;

    // Three decorrelated noise fields (offset by large constants)
    let dn3_dy = (noise3d(sx + 71.235, sy + eps + 13.579, sz + 93.147) - noise3d(sx + 71.235, sy - eps + 13.579, sz + 93.147)) / (2.0 * eps);
    let dn2_dz = (noise3d(sx + 31.416, sy + 47.853, sz + eps + 12.734) - noise3d(sx + 31.416, sy + 47.853, sz - eps + 12.734)) / (2.0 * eps);
    let dn1_dz = (noise3d(sx, sy, sz + eps) - noise3d(sx, sy, sz - eps)) / (2.0 * eps);
    let dn3_dx = (noise3d(sx + eps + 71.235, sy + 13.579, sz + 93.147) - noise3d(sx - eps + 71.235, sy + 13.579, sz + 93.147)) / (2.0 * eps);
    let dn2_dx = (noise3d(sx + eps + 31.416, sy + 47.853, sz + 12.734) - noise3d(sx - eps + 31.416, sy + 47.853, sz + 12.734)) / (2.0 * eps);
    let dn1_dy = (noise3d(sx, sy + eps, sz) - noise3d(sx, sy - eps, sz)) / (2.0 * eps);

    return vec3<f32>(dn3_dy - dn2_dz, dn1_dz - dn3_dx, dn2_dx - dn1_dy);
}

// ---- Velocity grid compute ----
@compute @workgroup_size(4, 4, 4)
fn cs_velocity(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= GRID_X || id.y >= GRID_Y || id.z >= GRID_Z) { return; }

    let world_x = X_MIN + f32(id.x) / f32(GRID_X - 1u) * (X_MAX - X_MIN);
    let world_y = Y_MIN + f32(id.y) / f32(GRID_Y - 1u) * (Y_MAX - Y_MIN);
    let world_z = Z_MIN + f32(id.z) / f32(GRID_Z - 1u) * (Z_MAX - Z_MIN);

    let v1 = curl_noise_3d(world_x, world_y, world_z, u.noise_scale, u.time);
    let v2 = curl_noise_3d(world_x, world_y, world_z, u.noise_scale * 2.3, u.time * 1.7);
    var vel = (v1 + v2 * 0.4) * u.speed;
    vel.z *= 0.5;

    let idx = id.x + id.y * GRID_X + id.z * GRID_X * GRID_Y;
    velocity_grid[idx] = vec4<f32>(vel, 0.0);
}

// ---- Helpers ----
fn sample_velocity(pos: vec3<f32>) -> vec3<f32> {
    let gx = clamp((pos.x - X_MIN) / (X_MAX - X_MIN) * f32(GRID_X - 1u), 0.0, f32(GRID_X - 2u));
    let gy = clamp((pos.y - Y_MIN) / (Y_MAX - Y_MIN) * f32(GRID_Y - 1u), 0.0, f32(GRID_Y - 2u));
    let gz = clamp((pos.z - Z_MIN) / (Z_MAX - Z_MIN) * f32(GRID_Z - 1u), 0.0, f32(GRID_Z - 2u));

    let ix = u32(floor(gx));
    let iy = u32(floor(gy));
    let iz = u32(floor(gz));
    let fx = gx - floor(gx);
    let fy = gy - floor(gy);
    let fz = gz - floor(gz);

    let i000 = ix + iy * GRID_X + iz * GRID_X * GRID_Y;
    let v000 = velocity_grid[i000].xyz;
    let v100 = velocity_grid[i000 + 1u].xyz;
    let v010 = velocity_grid[i000 + GRID_X].xyz;
    let v110 = velocity_grid[i000 + GRID_X + 1u].xyz;
    let v001 = velocity_grid[i000 + GRID_X * GRID_Y].xyz;
    let v101 = velocity_grid[i000 + GRID_X * GRID_Y + 1u].xyz;
    let v011 = velocity_grid[i000 + GRID_X * GRID_Y + GRID_X].xyz;
    let v111 = velocity_grid[i000 + GRID_X * GRID_Y + GRID_X + 1u].xyz;

    let vx0 = mix(v000, v100, vec3(fx));
    let vx1 = mix(v010, v110, vec3(fx));
    let vx2 = mix(v001, v101, vec3(fx));
    let vx3 = mix(v011, v111, vec3(fx));
    let vy0 = mix(vx0, vx1, vec3(fy));
    let vy1 = mix(vx2, vx3, vec3(fy));
    return mix(vy0, vy1, vec3(fz));
}

fn rotate_xyz(p: vec3<f32>, rot: vec3<f32>) -> vec3<f32> {
    let sx = sin(rot.x); let cx = cos(rot.x);
    let sy = sin(rot.y); let cy = cos(rot.y);
    let sz = sin(rot.z); let cz = cos(rot.z);
    let y1 = p.y * cx - p.z * sx;
    let z1 = p.y * sx + p.z * cx;
    let x2 = p.x * cy + z1 * sy;
    let z2 = -p.x * sy + z1 * cy;
    let x3 = x2 * cz - y1 * sz;
    let y3 = x2 * sz + y1 * cz;
    return vec3(x3, y3, z2);
}

fn inverse_rotate(p: vec3<f32>, rot: vec3<f32>) -> vec3<f32> {
    let sx = sin(-rot.x); let cx = cos(-rot.x);
    let sy = sin(-rot.y); let cy = cos(-rot.y);
    let sz = sin(-rot.z); let cz = cos(-rot.z);
    let x1 = p.x * cz - p.y * sz;
    let y1 = p.x * sz + p.y * cz;
    let x2 = x1 * cy + p.z * sy;
    let z2 = -x1 * sy + p.z * cy;
    let y3 = y1 * cx - z2 * sx;
    let z3 = y1 * sx + z2 * cx;
    return vec3(x2, y3, z3);
}

fn point_in_tetra(p: vec3<f32>, ti: u32) -> bool {
    let base = ti * 4u;
    let v0 = u.tetra_verts[base].xyz;
    let v1 = u.tetra_verts[base + 1u].xyz;
    let v2 = u.tetra_verts[base + 2u].xyz;
    let v3 = u.tetra_verts[base + 3u].xyz;

    // Face 0: v0,v1,v2 — opposite v3
    var ab = v1 - v0; var ac = v2 - v0; var n = cross(ab, ac);
    var d_side = dot(v3 - v0, n); var p_side = dot(p - v0, n);
    if (d_side * p_side < 0.0) { return false; }

    // Face 1: v0,v2,v3 — opposite v1
    ab = v2 - v0; ac = v3 - v0; n = cross(ab, ac);
    d_side = dot(v1 - v0, n); p_side = dot(p - v0, n);
    if (d_side * p_side < 0.0) { return false; }

    // Face 2: v0,v1,v3 — opposite v2
    ab = v1 - v0; ac = v3 - v0; n = cross(ab, ac);
    d_side = dot(v2 - v0, n); p_side = dot(p - v0, n);
    if (d_side * p_side < 0.0) { return false; }

    // Face 3: v1,v2,v3 — opposite v0
    ab = v2 - v1; ac = v3 - v1; n = cross(ab, ac);
    d_side = dot(v0 - v1, n); p_side = dot(p - v1, n);
    if (d_side * p_side < 0.0) { return false; }

    return true;
}

// ---- Particle advection compute ----
@compute @workgroup_size(64)
fn cs_advect(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= u.active_count) { return; }

    var p = particles[idx];
    let life = p.pos_life.w;
    let stuck_to = i32(p.attrs.z);

    if (life <= 0.0 && stuck_to < 0) { return; }

    // Free all stuck particles (phase transition signal from CPU)
    if (u.free_all_stuck == 1u && stuck_to >= 0) {
        p.attrs.z = -1.0;
        p.trail_maxlife = vec4(p.pos_life.xyz, 6.0);
        p.pos_life.w = 6.0;
        particles[idx] = p;
        return;
    }

    // Save trail
    p.trail_maxlife = vec4(p.pos_life.xyz, p.trail_maxlife.w);

    if (stuck_to >= 0) {
        let ti = u32(stuck_to);
        let offset = p.stuck_offset.xyz;
        if (u.capture_phase == 2u) {
            let rotated = rotate_xyz(offset, u.assembled_rotation.xyz);
            p.pos_life = vec4(u.assembled_center.xyz + rotated, p.pos_life.w);
        } else {
            let rotated = rotate_xyz(offset, u.tetra_rotations[ti].xyz);
            p.pos_life = vec4(u.tetra_centers[ti].xyz + rotated, p.pos_life.w);
        }
        p.trail_maxlife = vec4(p.pos_life.xyz, p.trail_maxlife.w);
    } else {
        var vel = sample_velocity(p.pos_life.xyz);

        // Soft Z boundary
        if (p.pos_life.z < Z_MIN + 0.5) { vel.z += (Z_MIN + 0.5 - p.pos_life.z) * -2.0; }
        if (p.pos_life.z > Z_MAX - 0.2) { vel.z += (Z_MAX - 0.2 - p.pos_life.z) * -2.0; }

        p.pos_life = vec4(p.pos_life.xyz + vel * u.dt, p.pos_life.w - u.dt);

        // Tetra capture
        if (u.capture_enabled == 1u && p.pos_life.w > 0.0) {
            for (var ti = 0u; ti < 5u; ti++) {
                if (point_in_tetra(p.pos_life.xyz, ti)) {
                    p.attrs.z = f32(ti);
                    if (u.capture_phase == 2u) {
                        let rel = p.pos_life.xyz - u.assembled_center.xyz;
                        p.stuck_offset = vec4(inverse_rotate(rel, u.assembled_rotation.xyz), 0.0);
                    } else {
                        let rel = p.pos_life.xyz - u.tetra_centers[ti].xyz;
                        p.stuck_offset = vec4(inverse_rotate(rel, u.tetra_rotations[ti].xyz), 0.0);
                    }
                    break;
                }
            }
        }
    }

    particles[idx] = p;
}
"#;

const FLOW_RENDER_WGSL: &str = r#"
struct SceneUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct Particle {
    pos_life: vec4<f32>,
    trail_maxlife: vec4<f32>,
    attrs: vec4<f32>,
    stuck_offset: vec4<f32>,
};

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(1) @binding(0) var<storage, read> particles: array<Particle>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_pos: vec3<f32>,
};

fn hue_to_color(hue: f32) -> vec4<f32> {
    let h = fract(hue);
    let r = min(0.1 + 0.5 * abs(sin(h * 3.0)), 0.8);
    let g = min(0.15 + 0.4 * abs(sin(h * 3.0 + 1.0)), 0.7);
    let b = min(0.3 + 0.5 * abs(sin(h * 2.0 + 0.5)), 1.0);
    return vec4(r, g, b, 0.6);
}

@vertex
fn vs_particle(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let p = particles[iid];
    let life = p.pos_life.w;
    let max_life = p.trail_maxlife.w;
    let stuck_to = i32(p.attrs.z);

    // Dead particle — degenerate triangle behind camera
    if (life <= 0.0 && stuck_to < 0) {
        out.clip_position = vec4(0.0, 0.0, 2.0, 1.0);
        out.color = vec4(0.0);
        out.uv = vec2(0.0);
        out.world_pos = vec3(0.0);
        return out;
    }

    let t = clamp(life / max(max_life, 0.001), 0.0, 1.0);
    var alpha: f32;
    if (t > 0.9) { alpha = (1.0 - t) * 10.0; } else { alpha = sqrt(t); }

    let base_color = hue_to_color(p.attrs.x);
    let color = vec4(base_color.rgb, base_color.a * alpha * 0.7);
    let size = p.attrs.y * 0.5;

    var offsets = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
        vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
    );
    let offset = offsets[vid];
    let world_pos = vec3(p.pos_life.x + offset.x * size, p.pos_life.y + offset.y * size, p.pos_life.z);

    out.clip_position = scene.view_proj * vec4(world_pos, 1.0);
    out.color = color;
    out.uv = offset;
    out.world_pos = world_pos;
    return out;
}

struct OitOutput {
    @location(0) accum: vec4<f32>,
    @location(1) revealage: vec4<f32>,
};

@fragment
fn fs_particle(in: VertexOutput) -> OitOutput {
    let dist = length(in.uv);
    if (dist > 1.0) { discard; }
    let soft = 1.0 - dist * dist;
    let alpha = in.color.a * soft;
    if (alpha < 0.001) { discard; }

    let cam_dist = length(scene.camera_pos.xyz - in.world_pos);
    let d_norm = clamp(cam_dist / 40.0, 0.0, 1.0);
    let w = clamp(alpha * max(1e-2, 3e3 * pow(1.0 - d_norm, 4.0)), 1e-2, 3e3);

    var out: OitOutput;
    out.accum = vec4(in.color.rgb * alpha * w, alpha * w);
    out.revealage = vec4(alpha, 0.0, 0.0, 0.0);
    return out;
}
"#;

struct FlowFieldGpu {
    velocity_pipeline: wgpu::ComputePipeline,
    advect_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    render_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    velocity_grid_buffer: wgpu::Buffer,
    particle_buffer: wgpu::Buffer,
    spawn_cursor: usize,
    active_count: u32,
    pending_spawns: Vec<ParticleGpu>,
}

impl FlowFieldGpu {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, scene_bgl: &wgpu::BindGroupLayout) -> Self {
        // Buffers
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow_uniforms"),
            size: std::mem::size_of::<FlowUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let velocity_grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow_velocity_grid"),
            size: (32 * 32 * 8 * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flow_particles"),
            size: (MAX_PARTICLES * std::mem::size_of::<ParticleGpu>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute bind group layout + bind group
        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flow_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false, min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false, min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false, min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flow_compute_bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: velocity_grid_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: particle_buffer.as_entire_binding() },
            ],
        });

        // Compute pipelines
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flow_compute"),
            source: wgpu::ShaderSource::Wgsl(FLOW_COMPUTE_WGSL.into()),
        });
        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&compute_bgl], push_constant_ranges: &[],
        });
        let velocity_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("flow_velocity"), layout: Some(&compute_layout),
            module: &compute_shader, entry_point: Some("cs_velocity"),
            compilation_options: Default::default(), cache: None,
        });
        let advect_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("flow_advect"), layout: Some(&compute_layout),
            module: &compute_shader, entry_point: Some("cs_advect"),
            compilation_options: Default::default(), cache: None,
        });

        // Render bind group layout + bind group
        let render_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flow_render_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false, min_binding_size: None,
                },
                count: None,
            }],
        });
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flow_render_bg"),
            layout: &render_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() }],
        });

        // Render pipeline (OIT-compatible)
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flow_render"),
            source: wgpu::ShaderSource::Wgsl(FLOW_RENDER_WGSL.into()),
        });
        let render_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[scene_bgl, &render_bgl], push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("flow_particle"),
            layout: Some(&render_layout),
            vertex: wgpu::VertexState {
                module: &render_shader, entry_point: Some("vs_particle"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_particle"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::One,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::Zero,
                                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::Zero,
                                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4, mask: !0, alpha_to_coverage_enabled: false,
            },
            multiview: None, cache: None,
        });

        FlowFieldGpu {
            velocity_pipeline, advect_pipeline, compute_bind_group,
            render_pipeline, render_bind_group,
            uniform_buffer, velocity_grid_buffer, particle_buffer,
            spawn_cursor: 0, active_count: 0,
            pending_spawns: Vec::with_capacity(64),
        }
    }

}

impl FlowField {
    fn build_gpu_uniforms(&self, dt: f32) -> FlowUniformsGpu {
        let mut tetra_verts = [[0.0f32; 4]; 20];
        let mut tetra_centers = [[0.0f32; 4]; 5];
        let mut tetra_rotations = [[0.0f32; 4]; 5];

        for (ti, t) in self.tetras.iter().enumerate() {
            tetra_centers[ti] = [t.x, t.y, t.z, 0.0];
            tetra_rotations[ti] = [t.ax, t.ay, t.az, 0.0];

            if let Some(ref av) = t.assembled_verts {
                for (vi, v) in av.iter().enumerate() {
                    tetra_verts[ti * 4 + vi] = [v[0], v[1], v[2], 0.0];
                }
            } else {
                for (vi, v) in TETRA_VERTS.iter().enumerate() {
                    let scaled = [v[0] * t.scale, v[1] * t.scale, v[2] * t.scale];
                    let rotated = t.rotate_point(scaled);
                    tetra_verts[ti * 4 + vi] = [
                        rotated[0] + t.x, rotated[1] + t.y, rotated[2] + t.z, 0.0,
                    ];
                }
            }
        }

        let active_count = self.gpu.as_ref().map_or(0, |g| g.active_count);
        FlowUniformsGpu {
            time: self.time,
            dt,
            noise_scale: 0.15 * self.turbulence,
            speed: 2.0 + self.energy * 6.0,
            active_count,
            capture_phase: match self.capture_phase {
                CapturePhase::Scatter => 0,
                CapturePhase::Attract => 1,
                CapturePhase::Assembled => 2,
                CapturePhase::Release => 3,
                CapturePhase::Pause => 4,
            },
            free_all_stuck: if self.gpu_free_all_stuck { 1 } else { 0 },
            capture_enabled: if self.capture_cooldown <= 0.0
                && (self.capture_phase == CapturePhase::Scatter
                    || self.capture_phase == CapturePhase::Assembled)
            { 1 } else { 0 },
            assembled_center: [
                self.assembled_center[0], self.assembled_center[1],
                self.assembled_center[2], 0.0,
            ],
            assembled_rotation: [
                self.assembled_rotation[0], self.assembled_rotation[1],
                self.assembled_rotation[2], 0.0,
            ],
            phase_timer: self.phase_timer,
            _pad: [0.0; 3],
            tetra_verts,
            tetra_centers,
            tetra_rotations,
        }
    }
}

impl GpuEffect for FlowField {
    fn create_gpu_resources(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene_bgl: &wgpu::BindGroupLayout) {
        self.gpu = Some(FlowFieldGpu::new(device, queue, scene_bgl));
    }

    fn compute(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, audio: &AudioFrame) {
        let uniforms = self.build_gpu_uniforms(audio.dt);
        let gpu = self.gpu.as_mut().unwrap();

        // Upload new particles (wrap around — with 49K buffer and ~250 spawns/sec,
        // wrapping happens after ~196s, far longer than any particle lifetime)
        let spawns = std::mem::take(&mut gpu.pending_spawns);
        if !spawns.is_empty() {
            let particle_size = std::mem::size_of::<ParticleGpu>();
            for p in &spawns {
                let slot = gpu.spawn_cursor % MAX_PARTICLES;
                let offset = (slot * particle_size) as u64;
                queue.write_buffer(&gpu.particle_buffer, offset, bytemuck::cast_slice(std::slice::from_ref(p)));
                gpu.spawn_cursor += 1;
            }
            gpu.active_count = gpu.active_count.max((gpu.spawn_cursor.min(MAX_PARTICLES)) as u32);
        }

        // Upload uniforms (with updated active_count)
        let mut final_uniforms = uniforms;
        final_uniforms.active_count = gpu.active_count;
        queue.write_buffer(&gpu.uniform_buffer, 0, bytemuck::cast_slice(&[final_uniforms]));

        // Dispatch velocity grid compute (32/4=8, 32/4=8, 8/4=2)
        {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&gpu.velocity_pipeline);
            cpass.set_bind_group(0, &gpu.compute_bind_group, &[]);
            cpass.dispatch_workgroups(8, 8, 2);
        }

        // Dispatch particle advection
        if gpu.active_count > 0 {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&gpu.advect_pipeline);
            cpass.set_bind_group(0, &gpu.compute_bind_group, &[]);
            cpass.dispatch_workgroups((gpu.active_count + 63) / 64, 1, 1);
        }

        self.gpu_free_all_stuck = false;
    }

    fn render_gpu<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, scene_bg: &'a wgpu::BindGroup) {
        let gpu = match self.gpu.as_ref() {
            Some(g) if g.active_count > 0 => g,
            _ => return,
        };
        pass.set_pipeline(&gpu.render_pipeline);
        pass.set_bind_group(0, scene_bg, &[]);
        pass.set_bind_group(1, &gpu.render_bind_group, &[]);
        pass.draw(0..6, 0..gpu.active_count);
    }

    fn gpu_active(&self) -> bool { self.gpu.is_some() }
}
