// wgpu renderer — GPU state, scene render, bloom post-processing.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::drawing::Vertex;

// Shared WGSL: uniforms, vertex IO, vertex shader, and lighting function.
// Used by both the opaque scene shader and the OIT accumulation shader.
const SHARED_WGSL: &str = r#"
struct Uniforms { view_proj: mat4x4<f32>, camera_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>, @location(3) uv: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.normal = normal;
    out.world_pos = position;
    out.uv = uv;
    return out;
}

// Compute lit color for a fragment. Handles soft particles, HUD bypass, and full PBR lighting.
fn compute_lit_color(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);

    // Soft particle: radial falloff from quad center (uv = -1..1)
    if (in.uv.x != 0.0 || in.uv.y != 0.0) {
        let dist = length(in.uv);
        if (dist > 1.0) { discard; }
        let soft = 1.0 - dist * dist;
        return vec4<f32>(in.color.rgb, in.color.a * soft);
    }

    // Skip lighting for HUD elements (normal = 0,0,1 and z near 0)
    if (n.z > 0.99 && in.world_pos.z < 0.1) {
        return in.color;
    }

    // Directional light from upper-front-right
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 0.8));
    let ambient = 0.25;
    let ndotl = max(dot(n, light_dir), 0.0);
    let diffuse = ndotl * 0.55;

    // Per-pixel view direction from camera position
    let view_dir = normalize(u.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let ndotv = max(dot(n, view_dir), 0.001);
    let ndoth = max(dot(n, half_dir), 0.0);

    // GGX specular (Cook-Torrance BRDF)
    let roughness = 0.3;
    let a = roughness * roughness;
    let a2 = a * a;
    let d = ndoth * ndoth * (a2 - 1.0) + 1.0;
    let D = a2 / (3.14159 * d * d);
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    let G = (ndotv / (ndotv * (1.0 - k) + k)) * (ndotl / (ndotl * (1.0 - k) + k));
    let F = 0.04 + 0.96 * pow(1.0 - max(dot(n, half_dir), 0.0), 5.0);
    let spec = D * G * F / max(4.0 * ndotv * ndotl, 0.001);

    // Environment reflection
    let refl = reflect(-view_dir, n);
    let env_y = refl.y * 0.5 + 0.5;
    let env_base = mix(
        vec3<f32>(0.02, 0.02, 0.06),
        vec3<f32>(0.15, 0.20, 0.35),
        smoothstep(0.0, 1.0, env_y)
    );
    let horizon = exp(-16.0 * (env_y - 0.5) * (env_y - 0.5)) * vec3<f32>(0.12, 0.06, 0.02);
    let env_color = env_base + horizon;
    let env_fresnel = 0.04 + 0.96 * pow(1.0 - ndotv, 5.0);
    let reflection = env_color * env_fresnel * 0.8;

    // Subtle rim light
    let rim = pow(1.0 - ndotv, 4.0) * 0.10;

    let lit = in.color.rgb * (ambient + diffuse) + vec3<f32>(spec, spec, spec) + reflection;
    return vec4<f32>(lit + in.color.rgb * rim, in.color.a);
}
"#;

// Opaque scene fragment shader — just calls the shared lighting function.
const SCENE_SHADER: &str = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return compute_lit_color(in);
}
"#;

// View-projection uniform
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4], // xyz + padding for alignment
}

impl Uniforms {
    pub fn identity() -> Self {
        Uniforms {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            camera_pos: [0.0, 0.0, 16.0, 0.0],
        }
    }
}

// --- Matrix math ---
pub fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                r[j][i] += a[k][i] * b[j][k]; // column-major multiplication
            }
        }
    }
    r
}

/// Perspective projection (column-major for wgpu)
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y / 2.0).tan();
    let nf = 1.0 / (near - far);
    // wgpu uses column-major, Z range [0,1]
    [
        [f / aspect, 0.0, 0.0, 0.0],  // column 0
        [0.0, f, 0.0, 0.0],            // column 1
        [0.0, 0.0, far * nf, -1.0],    // column 2
        [0.0, 0.0, far * near * nf, 0.0], // column 3
    ]
}

/// Look-at view matrix (column-major for wgpu)
pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize3(sub3(target, eye));
    let s = normalize3(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],     // column 0
        [s[1], u[1], -f[1], 0.0],     // column 1
        [s[2], u[2], -f[2], 0.0],     // column 2
        [-dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0], // column 3
    ]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0]-b[0], a[1]-b[1], a[2]-b[2]] }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
}
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 { a[0]*b[0]+a[1]*b[1]+a[2]*b[2] }
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0]*v[0]+v[1]*v[1]+v[2]*v[2]).sqrt();
    if l < 1e-10 { return v; }
    [v[0]/l, v[1]/l, v[2]/l]
}

// Journey transition: radial warp toward screen center (singularity effect)
const WARP_SHADER: &str = r#"
struct WarpUniforms { intensity: f32, _pad: f32, _pad2: f32, _pad3: f32 };
@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> warp_u: WarpUniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) idx: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = uv[idx];
    return out;
}

@fragment
fn fs_warp(in: VsOut) -> @location(0) vec4<f32> {
    let center = vec2<f32>(0.5, 0.5);
    let uv = in.uv;
    let to_center = center - uv;
    let dist = length(to_center);
    let dir = normalize(to_center);

    // Warp UV toward center — stronger at edges, accelerates with intensity
    let warp = warp_u.intensity * warp_u.intensity; // quadratic for dramatic acceleration
    let pull = warp * dist * 1.8; // edges get pulled harder
    let warped_uv = uv + dir * pull;

    // Radial blur — sample multiple points along the pull direction
    var color = vec3<f32>(0.0);
    let blur_samples = 8;
    let blur_spread = warp * 0.03;
    for (var i = 0; i < blur_samples; i++) {
        let t = f32(i) / f32(blur_samples - 1);
        let sample_uv = warped_uv + dir * blur_spread * (t - 0.5);
        let clamped = clamp(sample_uv, vec2(0.0), vec2(1.0));
        color += textureSample(scene_tex, tex_sampler, clamped).rgb;
    }
    color /= f32(blur_samples);

    // Vignette darkening — edges go black as warp intensifies
    let vignette = 1.0 - warp * dist * 2.0;
    color *= max(vignette, 0.0);

    // Overall darken toward black at peak
    let darken = 1.0 - warp * 0.7;
    color *= max(darken, 0.0);

    // ACES tonemap (same as bloom shader — HDR → displayable)
    let a = color * (color * 2.51 + vec3<f32>(0.03));
    let b = color * (color * 2.43 + vec3<f32>(0.59)) + vec3<f32>(0.14);
    let tonemapped = a / b;

    return vec4<f32>(tonemapped, 1.0);
}
"#;

// Single-pass bloom: extract bright + box blur + composite in one fragment shader
const BLOOM_SHADER: &str = r#"
struct BloomUniforms { color_grade: vec4<f32> };
@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;
@group(0) @binding(2) var<uniform> bloom_u: BloomUniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) idx: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = uv[idx];
    return out;
}

@fragment
fn fs_bloom_composite(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, tex_sampler, in.uv);
    let tex_size = vec2<f32>(textureDimensions(scene_tex));
    let pixel = 1.0 / tex_size;

    // Extract bright areas and apply box blur in one pass
    var bloom = vec3<f32>(0.0);
    let radius = 4;
    let step = 2.0; // sample every 2 pixels for wider blur
    var count = 0.0;
    for (var x = -radius; x <= radius; x++) {
        for (var y = -radius; y <= radius; y++) {
            let offset = vec2<f32>(f32(x) * step, f32(y) * step) * pixel;
            let s = textureSample(scene_tex, tex_sampler, in.uv + offset);
            let brightness = dot(s.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
            let thresh = smoothstep(0.35, 0.7, brightness);
            bloom += s.rgb * thresh;
            count += 1.0;
        }
    }
    bloom /= count;

    let bloom_strength = 0.6;
    let composited = scene.rgb + bloom * bloom_strength;
    // Color grading — per-theme color temperature
    let graded = composited * bloom_u.color_grade.rgb;
    // ACES tonemap — compress HDR to displayable range
    let a = graded * (graded * 2.51 + vec3<f32>(0.03));
    let b = graded * (graded * 2.43 + vec3<f32>(0.59)) + vec3<f32>(0.14);
    let tonemapped = a / b;
    return vec4<f32>(tonemapped, scene.a);
}
"#;

// OIT accumulation fragment shader — uses shared lighting, adds OIT weighting.
const OIT_SHADER: &str = r#"
struct OitOutput {
    @location(0) accum: vec4<f32>,
    @location(1) revealage: vec4<f32>,
};

@fragment
fn fs_oit(in: VertexOutput) -> OitOutput {
    let lit = compute_lit_color(in);
    let alpha = lit.a;

    // Depth-based weight using linear camera distance
    let cam_dist = length(u.camera_pos.xyz - in.world_pos);
    let d_norm = clamp(cam_dist / 40.0, 0.0, 1.0);
    let w = clamp(alpha * max(1e-2, 3e3 * pow(1.0 - d_norm, 4.0)), 1e-2, 3e3);

    var out: OitOutput;
    out.accum = vec4<f32>(lit.rgb * alpha * w, alpha * w);
    out.revealage = vec4<f32>(alpha, 0.0, 0.0, 0.0);
    return out;
}
"#;

// OIT composite shader — blends accumulated transparency over the opaque scene
const OIT_COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0) var accum_tex: texture_2d<f32>;
@group(0) @binding(1) var reveal_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) idx: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = uv[idx];
    return out;
}

@fragment
fn fs_oit_composite(in: VsOut) -> @location(0) vec4<f32> {
    let accum = textureSample(accum_tex, tex_sampler, in.uv);
    let revealage = textureSample(reveal_tex, tex_sampler, in.uv).r;

    // If no transparent fragments were written, revealage stays at 1.0 (fully transparent)
    if (revealage >= 0.999) {
        discard;
    }

    // Average color = accumulated premultiplied color / accumulated weight
    let avg_color = accum.rgb / max(accum.a, 1e-5);

    // Output with standard alpha blending over the opaque scene
    return vec4<f32>(avg_color, 1.0 - revealage);
}
"#;

/// Reusable GPU buffer pair (vertex + index) that grows as needed.
/// Avoids per-frame allocation by reusing buffers when capacity is sufficient.
struct GpuBufferPair {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    vertex_capacity: usize, // in bytes
    index_capacity: usize,  // in bytes
}

impl GpuBufferPair {
    fn new(device: &wgpu::Device, label: &str, initial_verts: usize, initial_indices: usize) -> Self {
        let vb_size = (initial_verts * std::mem::size_of::<Vertex>()).max(16);
        let ib_size = (initial_indices * std::mem::size_of::<u32>()).max(16);
        GpuBufferPair {
            vertex: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{}_vb", label)),
                size: vb_size as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            index: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{}_ib", label)),
                size: ib_size as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            vertex_capacity: vb_size,
            index_capacity: ib_size,
        }
    }

    /// Upload data, reallocating if capacity is exceeded.
    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, verts: &[Vertex], indices: &[u32]) {
        let vb_bytes = bytemuck::cast_slice::<Vertex, u8>(verts);
        let ib_bytes = bytemuck::cast_slice::<u32, u8>(indices);

        if vb_bytes.len() > self.vertex_capacity {
            self.vertex_capacity = (vb_bytes.len() * 3 / 2).max(1024); // grow by 1.5x
            self.vertex = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: self.vertex_capacity as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if ib_bytes.len() > self.index_capacity {
            self.index_capacity = (ib_bytes.len() * 3 / 2).max(512);
            self.index = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: self.index_capacity as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if !vb_bytes.is_empty() { queue.write_buffer(&self.vertex, 0, vb_bytes); }
        if !ib_bytes.is_empty() { queue.write_buffer(&self.index, 0, ib_bytes); }
    }
}

/// A single fullscreen post-processing pass (e.g., bloom, vignette, chromatic aberration).
struct PostProcessPass {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
}

/// Chain of post-processing passes with ping-pong textures.
/// Reads from scene_texture, applies passes sequentially, final pass writes to surface.
struct PostProcessChain {
    passes: Vec<PostProcessPass>,
    bind_group_layout: wgpu::BindGroupLayout,
    // Ping-pong intermediate texture (only needed with 2+ passes)
    ping_texture: wgpu::TextureView,
}

impl PostProcessChain {
    fn create_ping_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("postprocess_ping"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view(&Default::default())
    }

    /// Execute all passes. Reads from `scene_texture`, writes final result to `surface_view`.
    fn execute(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sampler: &wgpu::Sampler,
        scene_texture: &wgpu::TextureView,
        surface_view: &wgpu::TextureView,
    ) {
        let n = self.passes.len();
        if n == 0 { return; }

        for (i, pass) in self.passes.iter().enumerate() {
            let is_last = i == n - 1;
            // Input texture: scene_texture for first pass, ping_texture for subsequent
            let input = if i == 0 { scene_texture } else { &self.ping_texture };
            // Output: surface_view for last pass, ping_texture for intermediate
            // (For single-pass chains, this reads scene_texture and writes to surface directly)

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: pass.uniform_buffer.as_entire_binding() },
                ],
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            {
                if is_last {
                    // Final pass writes to surface (no MSAA, sRGB format)
                    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("postprocess_final"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        ..Default::default()
                    });
                    rp.set_pipeline(&pass.pipeline);
                    rp.set_bind_group(0, &bind_group, &[]);
                    rp.draw(0..3, 0..1);
                } else {
                    // Intermediate pass writes to ping texture (HDR format)
                    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("postprocess_intermediate"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.ping_texture,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        ..Default::default()
                    });
                    rp.set_pipeline(&pass.pipeline);
                    rp.set_bind_group(0, &bind_group, &[]);
                    rp.draw(0..3, 0..1);
                }
            }
            queue.submit(std::iter::once(encoder.finish()));
        }
    }

    fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.ping_texture = Self::create_ping_texture(device, config);
    }
}

// Mandelbrot fractal zoom — GPU fragment shader
const MANDELBROT_SHADER: &str = r#"
struct MandelbrotUniforms {
    center: vec2<f32>,
    zoom: f32,
    time: f32,
    max_iter: u32,
    color_offset: f32,
    aspect: f32,
    num_colors: u32,
    palette: array<vec4<f32>, 6>,
};
@group(0) @binding(0) var<uniform> m: MandelbrotUniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_mandelbrot(@builtin(vertex_index) idx: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = uv[idx];
    return out;
}

@fragment
fn fs_mandelbrot(in: VsOut) -> @location(0) vec4<f32> {
    let scale = 2.0 / m.zoom;
    let cr = m.center.x + (in.uv.x - 0.5) * scale * m.aspect;
    let ci = m.center.y + (in.uv.y - 0.5) * scale;

    var zr = 0.0;
    var zi = 0.0;
    var iter = 0u;
    let max_i = m.max_iter;

    // Cardioid/bulb check
    let q = (cr - 0.25) * (cr - 0.25) + ci * ci;
    let in_cardioid = q * (q + (cr - 0.25)) < 0.25 * ci * ci;
    let dx = cr + 1.0;
    let in_bulb = dx * dx + ci * ci < 0.0625;

    if (in_cardioid || in_bulb) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    loop {
        if (iter >= max_i) { break; }
        let zr2 = zr * zr;
        let zi2 = zi * zi;
        if (zr2 + zi2 > 4.0) { break; }
        zi = 2.0 * zr * zi + ci;
        zr = zr2 - zi2 + cr;
        iter += 1u;
    }

    if (iter == max_i) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Smooth iteration count for anti-banding
    let zr2 = zr * zr;
    let zi2 = zi * zi;
    let log_zn = log2(max(zr2 + zi2, 0.0001)) * 0.5;
    let smooth_iter = f32(iter) + 1.0 - log2(max(log_zn, 0.001));

    // Color mapping — sample from palette with smooth interpolation
    let t = fract(smooth_iter / f32(max_i) * 3.0 + m.color_offset);
    let n = f32(m.num_colors);
    let idx_f = t * n;
    let idx0 = u32(floor(idx_f)) % m.num_colors;
    let idx1 = (idx0 + 1u) % m.num_colors;
    let frac = fract(idx_f);
    let c0 = m.palette[idx0].rgb;
    let c1 = m.palette[idx1].rgb;
    let color = mix(c0, c1, frac);

    return vec4<f32>(color, 1.0);
}
"#;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MandelbrotUniforms {
    pub center_re: f32,
    pub center_im: f32,
    pub zoom: f32,
    pub time: f32,
    pub max_iter: u32,
    pub color_offset: f32,
    pub aspect: f32,
    pub num_colors: u32,
    pub palette: [[f32; 4]; 6], // up to 6 colors (RGBA, A unused)
}

const SAMPLE_COUNT: u32 = 4;
const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const REVEALAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// Draw command for a GPU effect rendering in the OIT pass.
/// The renderer sets scene uniforms at group 0; this provides the rest.
pub struct GpuOitDrawCmd<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub bind_group_1: &'a wgpu::BindGroup,
    pub instances: u32,
}

pub struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scene_pipeline: wgpu::RenderPipeline,          // opaque with depth write
    oit_accum_pipeline: wgpu::RenderPipeline,       // OIT accumulation (dual targets)
    oit_composite_pipeline: wgpu::RenderPipeline,   // OIT compositing (fullscreen quad)
    scene_pipeline_no_depth: wgpu::RenderPipeline,  // HUD overlay, no depth
    msaa_texture: wgpu::TextureView,
    scene_texture: wgpu::TextureView,
    depth_texture: wgpu::TextureView,
    oit_accum_msaa: wgpu::TextureView,
    oit_accum_resolve: wgpu::TextureView,
    oit_reveal_msaa: wgpu::TextureView,
    oit_reveal_resolve: wgpu::TextureView,
    oit_composite_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    post_process: PostProcessChain,
    uniform_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    // Mandelbrot background
    mandelbrot_pipeline: wgpu::RenderPipeline,
    mandelbrot_uniform_buffer: wgpu::Buffer,
    mandelbrot_bgl: wgpu::BindGroupLayout,
    pub mandelbrot_active: bool,
    // Journey transition warp
    warp_pipeline: wgpu::RenderPipeline,
    warp_uniform_buffer: wgpu::Buffer,
    pub warp_intensity: f32,
    // Scene bind group + layout (exposed for GpuEffect pipeline creation)
    scene_bind_group_layout: wgpu::BindGroupLayout,
    // Persistent GPU buffers — reused across frames, grown as needed
    opaque_bufs: GpuBufferPair,
    transparent_bufs: GpuBufferPair,
    hud_bufs: GpuBufferPair,
}

impl GpuState {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance.create_surface(window).expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })).expect("request adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(), None,
        )).expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bind group layout for post-processing (1 texture + 1 sampler)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Uniform buffer for view-projection matrix
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[Uniforms::identity()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scene_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene_bg"),
            layout: &scene_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Scene pipeline (with MSAA + uniform)
        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene_shader"), source: wgpu::ShaderSource::Wgsl(format!("{}{}", SHARED_WGSL, SCENE_SHADER).into()),
        });
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&scene_bind_group_layout], push_constant_ranges: &[],
        });
        let depth_stencil_state = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let msaa_state = wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        // Scene pipeline — opaque geometry with depth write + MSAA
        let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene_opaque"),
            layout: Some(&scene_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader, entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: Some(depth_stencil_state.clone()),
            multisample: msaa_state,
            multiview: None, cache: None,
        });

        // OIT accumulation pipeline — dual render targets, depth read-only
        let oit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oit_shader"), source: wgpu::ShaderSource::Wgsl(format!("{}{}", SHARED_WGSL, OIT_SHADER).into()),
        });
        let oit_accum_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oit_accum"),
            layout: Some(&scene_layout),
            vertex: wgpu::VertexState {
                module: &oit_shader, entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &oit_shader, entry_point: Some("fs_oit"),
                targets: &[
                    // Target 0: accumulation (additive blend)
                    Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
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
                    // Target 1: revealage (multiplicative: dst *= (1 - src))
                    Some(wgpu::ColorTargetState {
                        format: REVEALAGE_FORMAT,
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
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: false,
                ..depth_stencil_state
            }),
            multisample: msaa_state,
            multiview: None, cache: None,
        });

        // OIT composite pipeline — fullscreen quad, blends OIT result over opaque scene
        let oit_composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("oit_composite_shader"), source: wgpu::ShaderSource::Wgsl(OIT_COMPOSITE_SHADER.into()),
        });
        let oit_composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("oit_composite_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let oit_composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&oit_composite_bgl], push_constant_ranges: &[],
        });
        let oit_composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("oit_composite"),
            layout: Some(&oit_composite_layout),
            vertex: wgpu::VertexState {
                module: &oit_composite_shader, entry_point: Some("vs_full"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &oit_composite_shader, entry_point: Some("fs_oit_composite"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: msaa_state,
            multiview: None, cache: None,
        });

        // HUD pipeline — no depth testing (2D overlay)
        let scene_pipeline_no_depth = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&scene_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader, entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: msaa_state,
            multiview: None, cache: None,
        });

        // Bloom composite pipeline
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_shader"), source: wgpu::ShaderSource::Wgsl(BLOOM_SHADER.into()),
        });
        let bloom_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&bind_group_layout], push_constant_ranges: &[],
        });
        let bloom_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom"),
            layout: Some(&bloom_layout),
            vertex: wgpu::VertexState {
                module: &bloom_shader, entry_point: Some("vs_full"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &bloom_shader, entry_point: Some("fs_bloom_composite"),
                targets: &[Some(format.into())],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None, cache: None,
        });

        // Journey warp transition pipeline (reuses bloom bind group layout)
        let warp_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("warp_shader"), source: wgpu::ShaderSource::Wgsl(WARP_SHADER.into()),
        });
        let warp_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("warp"),
            layout: Some(&bloom_layout),
            vertex: wgpu::VertexState {
                module: &warp_shader, entry_point: Some("vs_full"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &warp_shader, entry_point: Some("fs_warp"),
                targets: &[Some(format.into())],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None, cache: None,
        });
        let warp_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("warp_uniforms"),
            contents: bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Mandelbrot background pipeline
        let mandelbrot_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mandelbrot_shader"), source: wgpu::ShaderSource::Wgsl(MANDELBROT_SHADER.into()),
        });
        let mandelbrot_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mandelbrot_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let mandelbrot_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&mandelbrot_bgl], push_constant_ranges: &[],
        });
        let mandelbrot_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mandelbrot"),
            layout: Some(&mandelbrot_layout),
            vertex: wgpu::VertexState {
                module: &mandelbrot_shader, entry_point: Some("vs_mandelbrot"),
                buffers: &[], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mandelbrot_shader, entry_point: Some("fs_mandelbrot"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: msaa_state,
            multiview: None, cache: None,
        });
        let mandelbrot_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mandelbrot_uniforms"),
            contents: bytemuck::cast_slice(&[MandelbrotUniforms {
                center_re: -0.7436439,
                center_im: 0.1318259,
                zoom: 1.0, time: 0.0, max_iter: 128, color_offset: 0.0,
                aspect: config.width as f32 / config.height.max(1) as f32,
                num_colors: 3,
                palette: [[1.0,0.0,0.0,0.0],[1.0,1.0,1.0,0.0],[0.0,0.0,0.0,0.0],
                           [0.0;4],[0.0;4],[0.0;4]],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Textures
        let msaa_texture = Self::create_msaa_texture(&device, &config);
        let scene_texture = Self::create_render_texture(&device, &config);
        let depth_texture = Self::create_depth_texture(&device, &config);
        let (oit_accum_msaa, oit_accum_resolve) = Self::create_oit_accum_textures(&device, &config);
        let (oit_reveal_msaa, oit_reveal_resolve) = Self::create_oit_reveal_textures(&device, &config);

        // Post-process chain (bloom is the first and currently only pass)
        let bloom_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bloom_uniforms"),
            contents: bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 1.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let post_process = PostProcessChain {
            passes: vec![PostProcessPass {
                pipeline: bloom_pipeline,
                uniform_buffer: bloom_uniform_buffer,
            }],
            bind_group_layout: bind_group_layout,
            ping_texture: PostProcessChain::create_ping_texture(&device, &config),
        };

        let opaque_bufs = GpuBufferPair::new(&device, "opaque", 4096, 8192);
        let transparent_bufs = GpuBufferPair::new(&device, "transparent", 8192, 16384);
        let hud_bufs = GpuBufferPair::new(&device, "hud", 4096, 8192);

        GpuState {
            surface, device, queue, config,
            scene_pipeline, oit_accum_pipeline, oit_composite_pipeline, scene_pipeline_no_depth,
            msaa_texture, scene_texture, depth_texture,
            oit_accum_msaa, oit_accum_resolve, oit_reveal_msaa, oit_reveal_resolve,
            oit_composite_bgl,
            sampler, post_process, uniform_buffer, scene_bind_group,
            scene_bind_group_layout,
            mandelbrot_pipeline, mandelbrot_uniform_buffer, mandelbrot_bgl,
            mandelbrot_active: false,
            warp_pipeline, warp_uniform_buffer,
            warp_intensity: 0.0,
            opaque_bufs, transparent_bufs, hud_bufs,
        }
    }

    fn create_msaa_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: SAMPLE_COUNT, dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&Default::default())
    }

    fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: SAMPLE_COUNT, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }).create_view(&Default::default())
    }

    fn create_render_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene_rt"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view(&Default::default())
    }

    fn create_oit_accum_textures(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> (wgpu::TextureView, wgpu::TextureView) {
        let size = wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 };
        let msaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oit_accum_msaa"), size, mip_level_count: 1, sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2, format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&Default::default());
        let resolve = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oit_accum_resolve"), size, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view(&Default::default());
        (msaa, resolve)
    }

    fn create_oit_reveal_textures(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> (wgpu::TextureView, wgpu::TextureView) {
        let size = wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 };
        let msaa = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oit_reveal_msaa"), size, mip_level_count: 1, sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2, format: REVEALAGE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&Default::default());
        let resolve = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oit_reveal_resolve"), size, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: REVEALAGE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view(&Default::default());
        (msaa, resolve)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.msaa_texture = Self::create_msaa_texture(&self.device, &self.config);
        self.scene_texture = Self::create_render_texture(&self.device, &self.config);
        self.depth_texture = Self::create_depth_texture(&self.device, &self.config);
        let (am, ar) = Self::create_oit_accum_textures(&self.device, &self.config);
        self.oit_accum_msaa = am;
        self.oit_accum_resolve = ar;
        let (rm, rr) = Self::create_oit_reveal_textures(&self.device, &self.config);
        self.oit_reveal_msaa = rm;
        self.oit_reveal_resolve = rr;
        self.post_process.resize(&self.device, &self.config);
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    pub fn size(&self) -> (f32, f32) {
        (self.config.width as f32, self.config.height as f32)
    }

    pub fn update_uniforms(&self, uniforms: &Uniforms) {
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Access the GPU device (for creating resources in GpuEffect::create_gpu_resources).
    pub fn device(&self) -> &wgpu::Device { &self.device }

    /// Access the GPU queue (for submitting compute work and buffer uploads).
    pub fn queue(&self) -> &wgpu::Queue { &self.queue }

    /// Access the scene bind group layout (for GpuEffect pipeline creation).
    pub fn scene_bgl(&self) -> &wgpu::BindGroupLayout { &self.scene_bind_group_layout }

    /// Submit a command encoder (for compute dispatches from GpuEffects).
    #[allow(dead_code)]
    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn set_color_grade(&self, grade: [f32; 3]) {
        let data = [grade[0], grade[1], grade[2], 1.0f32];
        // Write to the bloom pass uniform buffer (first pass in the chain)
        if let Some(bloom_pass) = self.post_process.passes.first() {
            self.queue.write_buffer(&bloom_pass.uniform_buffer, 0, bytemuck::cast_slice(&data));
        }
    }

    pub fn update_mandelbrot(&self, uniforms: &MandelbrotUniforms) {
        self.queue.write_buffer(&self.mandelbrot_uniform_buffer, 0, bytemuck::cast_slice(&[*uniforms]));
    }

    /// Render scene (opaque + transparent 3D) and HUD (2D overlay).
    /// Call update_uniforms with the 3D camera before this.
    /// `gpu_oit` provides an optional GPU effect draw command for the OIT pass.
    pub fn render(&mut self,
                  opaque_verts: &[Vertex], opaque_indices: &[u32],
                  transparent_verts: &[Vertex], transparent_indices: &[u32],
                  hud_verts: &[Vertex], hud_indices: &[u32],
                  gpu_oit: Option<&GpuOitDrawCmd>) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let surface_view = output.texture.create_view(&Default::default());

        // Upload geometry to persistent GPU buffers (grown as needed, reused across frames)
        self.opaque_bufs.upload(&self.device, &self.queue, opaque_verts, opaque_indices);
        self.transparent_bufs.upload(&self.device, &self.queue, transparent_verts, transparent_indices);
        self.hud_bufs.upload(&self.device, &self.queue, hud_verts, hud_indices);

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        // Pass 0 (optional): Mandelbrot background — fullscreen fractal
        if self.mandelbrot_active {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mandelbrot_bg"),
                layout: &self.mandelbrot_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.mandelbrot_uniform_buffer.as_entire_binding(),
                }],
            });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("mandelbrot"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.mandelbrot_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass 1: Opaque 3D scene (depth write ON, MSAA → resolve to scene_texture)
        {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                // Load if mandelbrot rendered, Clear if not
                let color_load = if self.mandelbrot_active {
                    wgpu::LoadOp::Load
                } else {
                    wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
                };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("scene_opaque"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: color_load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(depth_attachment),
                    ..Default::default()
                });
                pass.set_pipeline(&self.scene_pipeline);
                pass.set_bind_group(0, &self.scene_bind_group, &[]);
                if !opaque_indices.is_empty() {
                    pass.set_vertex_buffer(0, self.opaque_bufs.vertex.slice(..));
                    pass.set_index_buffer(self.opaque_bufs.index.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..opaque_indices.len() as u32, 0, 0..1);
                }
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass 2: OIT Accumulation (transparent geometry → accum + revealage targets)
        let has_transparent = !transparent_indices.is_empty() || gpu_oit.is_some();
        if has_transparent {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("oit_accum"),
                    color_attachments: &[
                        // Target 0: accumulation (clear to black/zero)
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.oit_accum_msaa,
                            resolve_target: Some(&self.oit_accum_resolve),
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        // Target 1: revealage (clear to white = fully transparent)
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.oit_reveal_msaa,
                            resolve_target: Some(&self.oit_reveal_resolve),
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                // CPU transparent geometry
                if !transparent_indices.is_empty() {
                    pass.set_pipeline(&self.oit_accum_pipeline);
                    pass.set_bind_group(0, &self.scene_bind_group, &[]);
                    pass.set_vertex_buffer(0, self.transparent_bufs.vertex.slice(..));
                    pass.set_index_buffer(self.transparent_bufs.index.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..transparent_indices.len() as u32, 0, 0..1);
                }
                // GPU effect particles (same OIT pass, different pipeline)
                if let Some(cmd) = gpu_oit {
                    pass.set_pipeline(cmd.pipeline);
                    pass.set_bind_group(0, &self.scene_bind_group, &[]);
                    pass.set_bind_group(1, cmd.bind_group_1, &[]);
                    pass.draw(0..6, 0..cmd.instances);
                }
            }
            self.queue.submit(std::iter::once(encoder.finish()));

            // Pass 2.5: OIT Composite (blend OIT result over opaque scene)
            let oit_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("oit_composite_bg"),
                layout: &self.oit_composite_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.oit_accum_resolve) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.oit_reveal_resolve) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("oit_composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // preserve opaque scene
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.oit_composite_pipeline);
                pass.set_bind_group(0, &oit_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass 3: HUD → scene_texture (with identity uniform, no depth)
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[Uniforms::identity()]));
        if !hud_indices.is_empty() {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("hud"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                // Full surface — no viewport restriction, background fills to edges
                pass.set_pipeline(&self.scene_pipeline_no_depth);
                pass.set_bind_group(0, &self.scene_bind_group, &[]);
                pass.set_vertex_buffer(0, self.hud_bufs.vertex.slice(..));
                pass.set_index_buffer(self.hud_bufs.index.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..hud_indices.len() as u32, 0, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Post-process: normal bloom, or warp transition when active
        // Both read from scene_texture (HDR) and write to surface_view (sRGB)
        if self.warp_intensity > 0.001 {
            // Warp replaces bloom during transition
            self.queue.write_buffer(&self.warp_uniform_buffer, 0,
                bytemuck::cast_slice(&[self.warp_intensity, 0.0f32, 0.0, 0.0]));
            let warp_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("warp_bg"),
                layout: &self.post_process.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.scene_texture) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.warp_uniform_buffer.as_entire_binding() },
                ],
            });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("warp_transition"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.warp_pipeline);
                pass.set_bind_group(0, &warp_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        } else {
            self.post_process.execute(
                &self.device, &self.queue, &self.sampler,
                &self.scene_texture, &surface_view,
            );
        }

        output.present();
    }
}
