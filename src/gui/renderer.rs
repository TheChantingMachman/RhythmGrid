// wgpu renderer — GPU state, scene render, bloom post-processing.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::drawing::Vertex;

const SCENE_SHADER: &str = r#"
struct Uniforms { view_proj: mat4x4<f32>, camera_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) color: vec4<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    out.normal = normal;
    out.world_pos = position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);

    // Skip lighting for HUD elements (normal = 0,0,1 and z near 0)
    if (n.z > 0.99 && in.world_pos.z < 0.1) {
        return in.color;
    }

    // Directional light from upper-front-right
    let light_dir = normalize(vec3<f32>(0.3, 0.5, 0.8));
    let ambient = 0.25;
    let diffuse = max(dot(n, light_dir), 0.0) * 0.55;

    // Per-pixel view direction from camera position
    let view_dir = normalize(u.camera_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.4;

    // Fresnel (Schlick approximation) — edges facing away from camera glow
    let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0) * 0.3;

    let brightness = ambient + diffuse + spec + fresnel;
    return vec4<f32>(in.color.rgb * brightness, in.color.a);
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

    pub fn from_mat(m: [[f32; 4]; 4]) -> Self {
        Uniforms { view_proj: m, camera_pos: [0.0, 0.0, 16.0, 0.0] }
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

// Single-pass bloom: extract bright + box blur + composite in one fragment shader
const BLOOM_SHADER: &str = r#"
@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

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
            let thresh = smoothstep(0.15, 0.4, brightness);
            bloom += s.rgb * thresh;
            count += 1.0;
        }
    }
    bloom /= count;

    let bloom_strength = 0.8;
    return vec4<f32>(scene.rgb + bloom * bloom_strength, scene.a);
}
"#;

const SAMPLE_COUNT: u32 = 4;

pub struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scene_pipeline: wgpu::RenderPipeline,         // opaque with depth write
    scene_pipeline_transparent: wgpu::RenderPipeline, // depth read, no write
    scene_pipeline_no_depth: wgpu::RenderPipeline, // HUD overlay, no depth
    msaa_texture: wgpu::TextureView,
    scene_texture: wgpu::TextureView,
    depth_texture: wgpu::TextureView,
    bloom_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bloom_bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
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
            label: Some("scene_shader"), source: wgpu::ShaderSource::Wgsl(SCENE_SHADER.into()),
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
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: Some(depth_stencil_state.clone()),
            multisample: msaa_state,
            multiview: None, cache: None,
        });

        // Transparent pipeline — depth read but no write + MSAA
        let scene_pipeline_transparent = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene_transparent"),
            layout: Some(&scene_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader, entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()], compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
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
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
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

        // Textures
        let msaa_texture = Self::create_msaa_texture(&device, &config);
        let scene_texture = Self::create_render_texture(&device, &config);
        let depth_texture = Self::create_depth_texture(&device, &config);

        GpuState {
            surface, device, queue, config,
            scene_pipeline, scene_pipeline_transparent, scene_pipeline_no_depth,
            msaa_texture, scene_texture, depth_texture,
            bloom_pipeline, sampler, bloom_bind_group_layout: bind_group_layout,
            uniform_buffer, scene_bind_group,
        }
    }

    fn create_msaa_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: SAMPLE_COUNT, dimension: wgpu::TextureDimension::D2,
            format: config.format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
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
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view(&Default::default())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.msaa_texture = Self::create_msaa_texture(&self.device, &self.config);
        self.scene_texture = Self::create_render_texture(&self.device, &self.config);
        self.depth_texture = Self::create_depth_texture(&self.device, &self.config);
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

    /// Render scene (opaque + transparent 3D) and HUD (2D overlay).
    /// Call update_uniforms with the 3D camera before this.
    pub fn render(&self,
                  opaque_verts: &[Vertex], opaque_indices: &[u32],
                  transparent_verts: &[Vertex], transparent_indices: &[u32],
                  hud_verts: &[Vertex], hud_indices: &[u32]) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let surface_view = output.texture.create_view(&Default::default());

        let opaque_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(opaque_verts), usage: wgpu::BufferUsages::VERTEX,
        });
        let opaque_ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(opaque_indices), usage: wgpu::BufferUsages::INDEX,
        });
        let trans_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(transparent_verts), usage: wgpu::BufferUsages::VERTEX,
        });
        let trans_ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(transparent_indices), usage: wgpu::BufferUsages::INDEX,
        });
        let hud_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(hud_verts), usage: wgpu::BufferUsages::VERTEX,
        });
        let hud_ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(hud_indices), usage: wgpu::BufferUsages::INDEX,
        });

        let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_texture,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        };

        // Pass 1: Opaque 3D scene (depth write ON, MSAA → resolve to scene_texture)
        {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("scene_opaque"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(depth_attachment),
                    ..Default::default()
                });
                pass.set_pipeline(&self.scene_pipeline);
                pass.set_bind_group(0, &self.scene_bind_group, &[]);
                if !opaque_indices.is_empty() {
                    pass.set_vertex_buffer(0, opaque_vb.slice(..));
                    pass.set_index_buffer(opaque_ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..opaque_indices.len() as u32, 0, 0..1);
                }
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass 2: Transparent 3D scene (depth read ON, write OFF)
        if !transparent_indices.is_empty() {
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("scene_transparent"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture,
                        resolve_target: Some(&self.scene_texture),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
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
                pass.set_pipeline(&self.scene_pipeline_transparent);
                pass.set_bind_group(0, &self.scene_bind_group, &[]);
                pass.set_vertex_buffer(0, trans_vb.slice(..));
                pass.set_index_buffer(trans_ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..transparent_indices.len() as u32, 0, 0..1);
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
                pass.set_vertex_buffer(0, hud_vb.slice(..));
                pass.set_index_buffer(hud_ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..hud_indices.len() as u32, 0, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass 3: Bloom composite — sample scene_texture → surface
        {
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None, layout: &self.bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.scene_texture) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bloom_composite"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.bloom_pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        output.present();
    }
}
