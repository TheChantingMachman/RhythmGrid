// Crystal fractal effect — recursive octahedra growing from faces.
// Each face of a parent octahedron can sprout a smaller child, creating
// dendritic crystal growth. OIT handles the overlapping transparency.

use super::{AudioEffect, AudioFrame, RenderContext, RenderPass};
use super::super::drawing::Vertex;

const NORMAL: [f32; 3] = [0.0, 0.0, 1.0];

// Elongated diamond — octahedron stretched along Y axis (tall crystal spire)
const STRETCH: f32 = 3.0;
const DIAMOND_VERTS: [[f32; 3]; 6] = [
    [ 1.0,  0.0,  0.0],          // +X (equator)
    [-1.0,  0.0,  0.0],          // -X (equator)
    [ 0.0,  0.0,  1.0],          // +Z (equator)
    [ 0.0,  0.0, -1.0],          // -Z (equator)
    [ 0.0,  STRETCH,  0.0],      // top point
    [ 0.0, -STRETCH,  0.0],      // bottom point
];

// 8 triangular faces (same topology as octahedron)
const DIAMOND_FACES: [[usize; 3]; 8] = [
    [0, 2, 4], [2, 1, 4], [1, 3, 4], [3, 0, 4], // top 4
    [2, 0, 5], [1, 2, 5], [3, 1, 5], [0, 3, 5], // bottom 4
];

// 12 edges
const DIAMOND_EDGES: [(usize, usize); 12] = [
    (0, 2), (2, 1), (1, 3), (3, 0), // equator ring
    (0, 4), (1, 4), (2, 4), (3, 4), // to top
    (0, 5), (1, 5), (2, 5), (3, 5), // to bottom
];

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

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt().max(0.0001);
    [v[0]/len, v[1]/len, v[2]/len]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1]*b[2] - a[2]*b[1],
        a[2]*b[0] - a[0]*b[2],
        a[0]*b[1] - a[1]*b[0],
    ]
}

#[derive(PartialEq)]
enum CrystalPhase {
    Growing,
    Paused,     // fully grown, brief hold before exploding
    Exploding,
}

struct Fragment {
    x: f32, y: f32, z: f32,
    vx: f32, vy: f32, vz: f32,
    ax: f32, ay: f32, az: f32,       // rotation angles
    spin: [f32; 3],                   // rotation speeds
    life: f32,
    size: f32,
    sticks: usize,                    // how many sticks (2-4)
    gen_from_top: usize,              // 0=outermost, higher=closer to root
}

pub struct Crystal {
    time: f32,
    angles: [f32; 3],
    energy: f32,
    phase: CrystalPhase,
    phase_timer: f32,      // countdown for current phase step
    display_depth: usize,  // how many generations to render
    max_depth: usize,
    fragments: Vec<Fragment>,
    rng_state: u64,
}

impl Crystal {
    pub fn new() -> Self {
        Crystal {
            time: 0.0,
            angles: [0.0; 3],
            energy: 0.0,
            phase: CrystalPhase::Growing,
            phase_timer: 3.0,
            display_depth: 0,
            max_depth: 4,
            fragments: Vec::new(),
            rng_state: 0xC2757A10CAFE,
        }
    }

    /// Emit one octahedron (faces + edges) at a given position, scale, and orientation.
    /// `basis` is a 3x3 rotation matrix (columns = right, up, forward) that orients the shape.
    fn emit_octahedron(
        verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
        center: [f32; 3], scale: f32, basis: [[f32; 3]; 3],
        face_color: [f32; 4], edge_color: [f32; 4],
    ) {
        // Transform octahedron vertices by basis and scale
        let transformed: Vec<[f32; 3]> = DIAMOND_VERTS.iter().map(|v| {
            let x = v[0] * basis[0][0] + v[1] * basis[1][0] + v[2] * basis[2][0];
            let y = v[0] * basis[0][1] + v[1] * basis[1][1] + v[2] * basis[2][1];
            let z = v[0] * basis[0][2] + v[1] * basis[1][2] + v[2] * basis[2][2];
            [x * scale + center[0], y * scale + center[1], z * scale + center[2]]
        }).collect();

        // Faces
        for face in &DIAMOND_FACES {
            let base = verts.len() as u32;
            for &vi in face {
                verts.push(Vertex {
                    position: transformed[vi],
                    normal: NORMAL,
                    color: face_color,
                    uv: [0.0, 0.0],
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2]);
        }

        // Edges
        let thickness = scale * 0.015;
        let cam_dir = [0.0f32, 0.0, 1.0];
        for &(a, b) in &DIAMOND_EDGES {
            let p0 = transformed[a];
            let p1 = transformed[b];
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let dz = p1[2] - p0[2];
            let c = cross([dx, dy, dz], cam_dir);
            let clen = (c[0]*c[0] + c[1]*c[1] + c[2]*c[2]).sqrt().max(0.001);
            let nx = c[0] / clen * thickness;
            let ny = c[1] / clen * thickness;
            let nz = c[2] / clen * thickness;
            let base = verts.len() as u32;
            verts.push(Vertex { position: [p0[0]+nx, p0[1]+ny, p0[2]+nz], normal: NORMAL, color: edge_color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p0[0]-nx, p0[1]-ny, p0[2]-nz], normal: NORMAL, color: edge_color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p1[0]-nx, p1[1]-ny, p1[2]-nz], normal: NORMAL, color: edge_color, uv: [0.0, 0.0] });
            verts.push(Vertex { position: [p1[0]+nx, p1[1]+ny, p1[2]+nz], normal: NORMAL, color: edge_color, uv: [0.0, 0.0] });
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }

    fn rand_f32(&mut self) -> f32 {
        self.rng_state = self.rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng_state >> 33) as f32) / (u32::MAX as f32 / 2.0)
    }

    /// Collect world-space centers of all children at exactly `target_depth` levels deep.
    fn collect_positions_at_depth(
        center: [f32; 3], scale: f32, basis: [[f32; 3]; 3],
        current_depth: usize, target_depth: usize, is_root: bool,
        out: &mut Vec<([f32; 3], f32)>, // (position, scale)
    ) {
        if current_depth == 0 { return; }

        let child_scale = scale * 0.443;
        let faces = if is_root { &DIAMOND_FACES[..] } else { &DIAMOND_FACES[..4] };

        for face in faces {
            let verts_3d: Vec<[f32; 3]> = face.iter().map(|&vi| {
                let v = DIAMOND_VERTS[vi];
                let x = v[0] * basis[0][0] + v[1] * basis[1][0] + v[2] * basis[2][0];
                let y = v[0] * basis[0][1] + v[1] * basis[1][1] + v[2] * basis[2][1];
                let z = v[0] * basis[0][2] + v[1] * basis[1][2] + v[2] * basis[2][2];
                [x * scale + center[0], y * scale + center[1], z * scale + center[2]]
            }).collect();

            let fc = [
                (verts_3d[0][0] + verts_3d[1][0] + verts_3d[2][0]) / 3.0,
                (verts_3d[0][1] + verts_3d[1][1] + verts_3d[2][1]) / 3.0,
                (verts_3d[0][2] + verts_3d[1][2] + verts_3d[2][2]) / 3.0,
            ];

            let to_face = [fc[0]-center[0], fc[1]-center[1], fc[2]-center[2]];
            let e1 = [verts_3d[1][0]-verts_3d[0][0], verts_3d[1][1]-verts_3d[0][1], verts_3d[1][2]-verts_3d[0][2]];
            let e2 = [verts_3d[2][0]-verts_3d[0][0], verts_3d[2][1]-verts_3d[0][1], verts_3d[2][2]-verts_3d[0][2]];
            let cn = cross(e1, e2);
            let dot = cn[0]*to_face[0] + cn[1]*to_face[1] + cn[2]*to_face[2];
            let face_normal = if dot < 0.0 {
                normalize([-cn[0], -cn[1], -cn[2]])
            } else {
                normalize(cn)
            };
            let up = face_normal;
            let right = normalize(e1);
            let forward = cross(right, up);
            let child_basis = [right, up, forward];

            if current_depth == 1 {
                // This is the target depth — collect this position
                out.push((fc, child_scale));
            } else {
                // Recurse deeper
                Self::collect_positions_at_depth(fc, child_scale, child_basis,
                    current_depth - 1, target_depth, false, out);
            }
        }
    }

    /// Emit a half-diamond (top pyramid only) — 4 triangular faces + square base.
    /// The base sits at y=0, the point extends along +Y (in local basis space).
    fn emit_half_diamond(
        verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
        center: [f32; 3], scale: f32, basis: [[f32; 3]; 3],
        face_color: [f32; 4], edge_color: [f32; 4],
    ) {
        // Half-diamond: 4 equator vertices + top point (no bottom point)
        let half_verts: [[f32; 3]; 5] = [
            [ 1.0,  0.0,  0.0],      // equator +X
            [-1.0,  0.0,  0.0],      // equator -X
            [ 0.0,  0.0,  1.0],      // equator +Z
            [ 0.0,  0.0, -1.0],      // equator -Z
            [ 0.0,  STRETCH, 0.0],   // top point
        ];
        // 4 triangular side faces
        let half_faces: [[usize; 3]; 4] = [
            [0, 2, 4], [2, 1, 4], [1, 3, 4], [3, 0, 4],
        ];
        // Square base (2 triangles)
        let base_faces: [[usize; 3]; 2] = [
            [0, 2, 1], [0, 1, 3],
        ];
        // Edges: equator ring + 4 to tip
        let half_edges: [(usize, usize); 8] = [
            (0, 2), (2, 1), (1, 3), (3, 0),
            (0, 4), (1, 4), (2, 4), (3, 4),
        ];

        let transformed: Vec<[f32; 3]> = half_verts.iter().map(|v| {
            let x = v[0] * basis[0][0] + v[1] * basis[1][0] + v[2] * basis[2][0];
            let y = v[0] * basis[0][1] + v[1] * basis[1][1] + v[2] * basis[2][1];
            let z = v[0] * basis[0][2] + v[1] * basis[1][2] + v[2] * basis[2][2];
            [x * scale + center[0], y * scale + center[1], z * scale + center[2]]
        }).collect();

        // Side faces
        for face in &half_faces {
            let base = verts.len() as u32;
            for &vi in face {
                verts.push(Vertex { position: transformed[vi], normal: NORMAL, color: face_color, uv: [0.0, 0.0] });
            }
            indices.extend_from_slice(&[base, base+1, base+2]);
        }
        // Base face (slightly more opaque to look solid)
        let base_color = [face_color[0], face_color[1], face_color[2], face_color[3] * 1.5];
        for face in &base_faces {
            let base = verts.len() as u32;
            for &vi in face {
                verts.push(Vertex { position: transformed[vi], normal: NORMAL, color: base_color, uv: [0.0, 0.0] });
            }
            indices.extend_from_slice(&[base, base+1, base+2]);
        }
        // Edges
        let thickness = scale * 0.015;
        let cam_dir = [0.0f32, 0.0, 1.0];
        for &(a, b) in &half_edges {
            let p0 = transformed[a];
            let p1 = transformed[b];
            let dx = p1[0]-p0[0]; let dy = p1[1]-p0[1]; let dz = p1[2]-p0[2];
            let c = cross([dx,dy,dz], cam_dir);
            let clen = (c[0]*c[0]+c[1]*c[1]+c[2]*c[2]).sqrt().max(0.001);
            let nx = c[0]/clen*thickness; let ny = c[1]/clen*thickness; let nz = c[2]/clen*thickness;
            let base = verts.len() as u32;
            verts.push(Vertex { position: [p0[0]+nx,p0[1]+ny,p0[2]+nz], normal: NORMAL, color: edge_color, uv: [0.0,0.0] });
            verts.push(Vertex { position: [p0[0]-nx,p0[1]-ny,p0[2]-nz], normal: NORMAL, color: edge_color, uv: [0.0,0.0] });
            verts.push(Vertex { position: [p1[0]-nx,p1[1]-ny,p1[2]-nz], normal: NORMAL, color: edge_color, uv: [0.0,0.0] });
            verts.push(Vertex { position: [p1[0]+nx,p1[1]+ny,p1[2]+nz], normal: NORMAL, color: edge_color, uv: [0.0,0.0] });
            indices.extend_from_slice(&[base,base+1,base+2,base,base+2,base+3]);
        }
    }

    /// Spawn half-diamond children on each face of a diamond at the given position/basis.
    /// Recurses: each child can sprout further children from its own side faces.
    fn spawn_children(
        verts: &mut Vec<Vertex>, indices: &mut Vec<u32>,
        center: [f32; 3], scale: f32, basis: [[f32; 3]; 3],
        face_color: [f32; 4], edge_color: [f32; 4],
        depth: usize,
        is_root: bool,
    ) {
        if depth == 0 { return; }

        let child_scale = scale * 0.443;
        let child_alpha = (face_color[3] * 0.9).max(0.05);
        let child_face = [face_color[0], face_color[1], face_color[2], child_alpha];

        // Root: all 8 faces. Half-diamond children: only top 4 faces.
        let faces = if is_root { &DIAMOND_FACES[..] } else { &DIAMOND_FACES[..4] };
        for face in faces {
            let verts_3d: Vec<[f32; 3]> = face.iter().map(|&vi| {
                let v = DIAMOND_VERTS[vi];
                let x = v[0] * basis[0][0] + v[1] * basis[1][0] + v[2] * basis[2][0];
                let y = v[0] * basis[0][1] + v[1] * basis[1][1] + v[2] * basis[2][1];
                let z = v[0] * basis[0][2] + v[1] * basis[1][2] + v[2] * basis[2][2];
                [x * scale + center[0], y * scale + center[1], z * scale + center[2]]
            }).collect();

            let fc = [
                (verts_3d[0][0] + verts_3d[1][0] + verts_3d[2][0]) / 3.0,
                (verts_3d[0][1] + verts_3d[1][1] + verts_3d[2][1]) / 3.0,
                (verts_3d[0][2] + verts_3d[1][2] + verts_3d[2][2]) / 3.0,
            ];

            // Normal pointing outward from center
            let to_face = [fc[0]-center[0], fc[1]-center[1], fc[2]-center[2]];
            let e1 = [verts_3d[1][0]-verts_3d[0][0], verts_3d[1][1]-verts_3d[0][1], verts_3d[1][2]-verts_3d[0][2]];
            let e2 = [verts_3d[2][0]-verts_3d[0][0], verts_3d[2][1]-verts_3d[0][1], verts_3d[2][2]-verts_3d[0][2]];
            let cn = cross(e1, e2);
            // Flip if pointing inward (dot with center-to-face should be positive)
            let dot = cn[0]*to_face[0] + cn[1]*to_face[1] + cn[2]*to_face[2];
            let face_normal = if dot < 0.0 {
                normalize([-cn[0], -cn[1], -cn[2]])
            } else {
                normalize(cn)
            };

            // Child basis derived from parent face edges — locked to the face, no sliding
            let up = face_normal;
            let right = normalize(e1); // first edge of the face
            let forward = cross(right, up); // perpendicular to both
            let child_basis = [right, up, forward];

            Self::emit_half_diamond(verts, indices, fc, child_scale, child_basis,
                child_face, edge_color);

            // Recurse from this child's position
            Self::spawn_children(verts, indices, fc, child_scale, child_basis,
                child_face, edge_color, depth - 1, false);
        }
    }
}

impl AudioEffect for Crystal {
    fn pass(&self) -> RenderPass { RenderPass::Transparent }

    fn update(&mut self, audio: &AudioFrame) {
        self.time += audio.dt;
        let target_energy = audio.bands.iter().sum::<f32>() / 7.0;
        self.energy += (target_energy - self.energy) * 3.0 * audio.dt;

        let energy_mod = 1.0 + self.energy * 2.0;
        self.angles[0] += 0.12 * audio.dt * energy_mod;
        self.angles[1] += 0.08 * audio.dt * energy_mod;
        self.angles[2] += 0.05 * audio.dt * energy_mod;

        // Update fragments
        for f in &mut self.fragments {
            f.x += f.vx * audio.dt;
            f.y += f.vy * audio.dt;
            f.z += f.vz * audio.dt;
            f.vy -= 0.5 * audio.dt;
            f.ax += f.spin[0] * audio.dt;
            f.ay += f.spin[1] * audio.dt;
            f.az += f.spin[2] * audio.dt;
            f.life -= audio.dt;
        }
        self.fragments.retain(|f| f.life > 0.0);

        // State machine
        self.phase_timer -= audio.dt;
        match self.phase {
            CrystalPhase::Growing => {
                if self.phase_timer <= 0.0 {
                    if self.display_depth < self.max_depth {
                        self.display_depth += 1;
                        self.phase_timer = 4.0; // next generation in 4 seconds
                    } else {
                        // Fully grown — pause before exploding
                        self.phase = CrystalPhase::Paused;
                        self.phase_timer = 3.0;
                    }
                }
            }
            CrystalPhase::Paused => {
                if self.phase_timer <= 0.0 {
                    self.phase = CrystalPhase::Exploding;
                    self.phase_timer = 1.5; // time between each generation exploding
                }
            }
            CrystalPhase::Exploding => {
                if self.phase_timer <= 0.0 {
                    if self.display_depth > 0 {
                        // Spawn explosion fragments for the outermost generation
                        let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
                        let basis = identity.map(|col| rotate_point(col, self.angles[0], self.angles[1], self.angles[2]));
                        let center = [5.0f32, -10.0, -2.0];
                        let mut positions = Vec::new();
                        Self::collect_positions_at_depth(
                            center, 4.0, basis,
                            self.display_depth, self.display_depth, true,
                            &mut positions,
                        );
                        // Scale fragment count and size by generation (larger = more/bigger)
                        // gen_from_top: 0 = outermost (smallest), higher = closer to root (larger)
                        let gen_from_top = self.max_depth - self.display_depth;
                        let count_mult = 1u32 << gen_from_top; // 1, 2, 4, 8, 16
                        let size_mult = (1u32 << gen_from_top) as f32; // 1, 2, 4, 8, 16

                        for (pos, s) in &positions {
                            let dir = normalize([pos[0] - center[0], pos[1] - center[1], pos[2] - center[2]]);
                            for _ in 0..(3 * count_mult) {
                                let r0 = self.rand_f32().abs();
                                let r1 = self.rand_f32().abs();
                                let r2 = self.rand_f32();
                                let r3 = self.rand_f32().abs();
                                let r4 = self.rand_f32();
                                let r5 = self.rand_f32();
                                let r6 = self.rand_f32();
                                let r7 = self.rand_f32().abs();
                                let r8 = self.rand_f32().abs();
                                let rs0 = self.rand_f32();
                                let rs1 = self.rand_f32();
                                let rs2 = self.rand_f32();
                                let ra0 = self.rand_f32().abs();
                                let ra1 = self.rand_f32().abs();
                                let ra2 = self.rand_f32().abs();
                                let angle = r0 * std::f32::consts::TAU;
                                let spread = 0.6 + r1 * 0.4;
                                let rx = angle.cos() * spread;
                                let ry = angle.sin() * spread;
                                let rz = r2 * 0.5;
                                let speed = 1.0 + r3 * 2.5;
                                let ox = r4 * s * 0.5;
                                let oy = r5 * s * 0.5;
                                let oz = r6 * s * 0.3;
                                self.fragments.push(Fragment {
                                    x: pos[0] + ox, y: pos[1] + oy, z: pos[2] + oz,
                                    vx: (dir[0] + rx) * speed,
                                    vy: (dir[1] + ry) * speed,
                                    vz: (dir[2] + rz) * speed,
                                    ax: ra0 * std::f32::consts::TAU,
                                    ay: ra1 * std::f32::consts::TAU,
                                    az: ra2 * std::f32::consts::TAU,
                                    spin: [2.0 + rs0 * 4.0, 3.0 + rs1 * 5.0, 1.0 + rs2 * 3.0],
                                    life: 2.5 + r7 * 1.5,
                                    size: s * (0.3 + r8 * 0.4) * size_mult,
                                    sticks: 2 + (r0 * 3.0) as usize,
                                    gen_from_top,
                                });
                            }
                        }
                        self.display_depth -= 1;
                        self.phase_timer = 1.5;
                    } else {
                        // All generations exploded — restart growth
                        self.phase = CrystalPhase::Growing;
                        self.phase_timer = 3.0;
                        self.display_depth = 0;
                    }
                }
            }
        }
    }

    fn render(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, _ctx: &RenderContext) {
        // White background
        let bg = 50.0;
        let base = verts.len() as u32;
        let bg_c = [1.0f32, 1.0, 1.0, 1.0];
        verts.push(Vertex { position: [-bg, bg, -5.0], normal: NORMAL, color: bg_c, uv: [0.0, 0.0] });
        verts.push(Vertex { position: [bg, bg, -5.0], normal: NORMAL, color: bg_c, uv: [0.0, 0.0] });
        verts.push(Vertex { position: [bg, -bg, -5.0], normal: NORMAL, color: bg_c, uv: [0.0, 0.0] });
        verts.push(Vertex { position: [-bg, -bg, -5.0], normal: NORMAL, color: bg_c, uv: [0.0, 0.0] });
        indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);

        // Build global rotation basis from angles
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let basis = identity.map(|col| rotate_point(col, self.angles[0], self.angles[1], self.angles[2]));

        let center = [5.0, -10.0, -2.0];
        let face_color = [0.01, 0.01, 0.03, 0.3];
        let edge_color = [0.0, 0.0, 0.0, 0.9];

        // Root: full diamond
        let scale = 4.0;
        Self::emit_octahedron(verts, indices, center, scale, basis, face_color, edge_color);

        // Children: half-diamonds sprouting from faces, recursively
        Self::spawn_children(verts, indices, center, scale, basis,
            face_color, edge_color, self.display_depth, true);

        // Explosion fragments — tumbling dark sticks
        let cam_dir = [0.0f32, 0.0, 1.0];
        for f in &self.fragments {
            if f.life <= 0.0 { continue; }
            let alpha = (f.life / 3.0).clamp(0.0, 1.0);
            let color = [0.0, 0.0, 0.0, alpha * 0.6];
            // Thinner sticks for larger/inner generations
            let thin_factor = 1.0 / (1.0 + f.gen_from_top as f32 * 1.5);
            let thickness = f.size * 0.04 * thin_factor;
            let half_len = f.size * 0.5;

            // Draw N sticks at different angles through the center, all tumbling together
            for i in 0..f.sticks {
                let stick_angle = (i as f32 / f.sticks as f32) * std::f32::consts::PI;
                let local = [
                    stick_angle.cos() * half_len,
                    stick_angle.sin() * half_len,
                    0.0,
                ];
                let tip = rotate_point(local, f.ax, f.ay, f.az);
                let neg = [-tip[0], -tip[1], -tip[2]];
                let p0 = [f.x + tip[0], f.y + tip[1], f.z + tip[2]];
                let p1 = [f.x + neg[0], f.y + neg[1], f.z + neg[2]];
                // Billboard perpendicular
                let dx = p1[0]-p0[0]; let dy = p1[1]-p0[1]; let dz = p1[2]-p0[2];
                let c = cross([dx,dy,dz], cam_dir);
                let clen = (c[0]*c[0]+c[1]*c[1]+c[2]*c[2]).sqrt().max(0.001);
                let nx = c[0]/clen*thickness;
                let ny = c[1]/clen*thickness;
                let nz = c[2]/clen*thickness;
                let base = verts.len() as u32;
                verts.push(Vertex { position: [p0[0]+nx,p0[1]+ny,p0[2]+nz], normal: NORMAL, color, uv: [0.0,0.0] });
                verts.push(Vertex { position: [p0[0]-nx,p0[1]-ny,p0[2]-nz], normal: NORMAL, color, uv: [0.0,0.0] });
                verts.push(Vertex { position: [p1[0]-nx,p1[1]-ny,p1[2]-nz], normal: NORMAL, color, uv: [0.0,0.0] });
                verts.push(Vertex { position: [p1[0]+nx,p1[1]+ny,p1[2]+nz], normal: NORMAL, color, uv: [0.0,0.0] });
                indices.extend_from_slice(&[base,base+1,base+2,base,base+2,base+3]);
            }
        }

        // Volumetric smoke — layered dark soft circles at varying depths
        // Heavy, smoky look like a photographic negative
        let fog_layers = 28;
        let fog_center_x = 5.0;
        let fog_center_y = -10.0;
        let hash = |n: f32| -> f32 {
            let s = (n * 127.1).sin() * 43758.5453;
            s - s.floor()
        };
        for layer in 0..fog_layers {
            let t = layer as f32 / fog_layers as f32;
            let z = -4.8 + t * 4.0;
            let seed = layer as f32 + self.time * 0.02;
            let ox = (hash(seed) - 0.5) * 20.0;
            let oy = (hash(seed + 7.3) - 0.5) * 20.0;
            let size = 4.0 + hash(seed + 13.7) * 12.0;
            let density = 0.06 + hash(seed + 23.1) * 0.12;

            let fx = fog_center_x + ox;
            let fy = fog_center_y + oy;
            let fog_color = [0.0f32, 0.0, 0.0, density];

            let base = verts.len() as u32;
            verts.push(Vertex { position: [fx - size, fy - size, z], normal: NORMAL, color: fog_color, uv: [-1.0, -1.0] });
            verts.push(Vertex { position: [fx + size, fy - size, z], normal: NORMAL, color: fog_color, uv: [ 1.0, -1.0] });
            verts.push(Vertex { position: [fx + size, fy + size, z], normal: NORMAL, color: fog_color, uv: [ 1.0,  1.0] });
            verts.push(Vertex { position: [fx - size, fy + size, z], normal: NORMAL, color: fog_color, uv: [-1.0,  1.0] });
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }
    }
}
