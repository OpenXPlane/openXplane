use crate::{
    hud::{HudVertex, MAX_VERTICES},
    scene::Scene,
};
use glam::{Mat4, Vec3};
use openxplane::obj8::Vertex;
use wgpu::util::DeviceExt;

pub struct Camera {
    pub target: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Camera {
    pub fn new(scene: &Scene) -> Self {
        Self {
            target: scene.center,
            radius: scene.radius,
            yaw: -0.7,
            pitch: 0.3,
            distance: scene.radius * 1.8,
        }
    }
    pub fn eye(&self) -> Vec3 {
        self.target
            + self.distance
                * Vec3::new(
                    self.yaw.sin() * self.pitch.cos(),
                    self.pitch.sin(),
                    -self.yaw.cos() * self.pitch.cos(),
                )
    }
    fn matrix(&self, width: u32, height: u32) -> Mat4 {
        Mat4::perspective_rh(
            45.0_f32.to_radians(),
            width as f32 / height as f32,
            self.radius * 0.01,
            self.radius * 100.0,
        ) * Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
    }
}

struct GpuMesh {
    vertex: wgpu::Buffer,
    count: u32,
    material: wgpu::BindGroup,
    glass: bool,
    center: Vec3,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    camera_buffer: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    opaque: wgpu::RenderPipeline,
    transparent: wgpu::RenderPipeline,
    meshes: Vec<GpuMesh>,
    format: wgpu::TextureFormat,
    clear: wgpu::Color,
    hud_pipeline: wgpu::RenderPipeline,
    hud_group: wgpu::BindGroup,
    hud_buffer: wgpu::Buffer,
    hud_count: u32,
}

pub struct Targets {
    depth: wgpu::TextureView,
    color: wgpu::TextureView,
    width: u32,
    height: u32,
}

pub fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())
}

impl Renderer {
    pub async fn new(
        adapter: &wgpu::Adapter,
        format: wgpu::TextureFormat,
        scene: &Scene,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let info = adapter.get_info();
        println!("GPU: {} ({:?})", info.name, info.backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("openXplane"),
                ..Default::default()
            })
            .await?;
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::cast_slice(&Mat4::IDENTITY.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("OBJ8 preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let pipeline = |glass| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if glass { "glass" } else { "opaque" }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    }],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(!glass),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 4,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: glass.then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let opaque = pipeline(false);
        let transparent = pipeline(true);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("color"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        // screen-space interface: font atlas, pipeline and a dynamic vertex buffer
        let atlas =
            image::load_from_memory(include_bytes!("../assets/font/roboto-mono-atlas.png"))?
                .to_rgba8();
        let atlas_size = wgpu::Extent3d {
            width: atlas.width(),
            height: atlas.height(),
            depth_or_array_layers: 1,
        };
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud font"),
            size: atlas_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.width() * 4),
                rows_per_image: Some(atlas.height()),
            },
            atlas_size,
        );
        let atlas_view = atlas_texture.create_view(&Default::default());
        let hud_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud font"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
        });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud"),
            bind_group_layouts: &[Some(&texture_layout)],
            immediate_size: 0,
        });
        let hud_attributes =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<HudVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &hud_attributes,
                }],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let hud_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<HudVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut meshes = Vec::new();
        for mesh in &scene.meshes {
            let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("OBJ8 triangles"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            let size = wgpu::Extent3d {
                width: mesh.texture.width(),
                height: mesh.texture.height(),
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("base color"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mesh.texture.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.width * 4),
                    rows_per_image: Some(size.height),
                },
                size,
            );
            let view = texture.create_view(&Default::default());
            let material = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material"),
                layout: &texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            meshes.push(GpuMesh {
                vertex,
                count: mesh.vertices.len().try_into()?,
                material,
                glass: mesh.glass,
                center: mesh.center,
            });
        }
        Ok(Self {
            device,
            queue,
            camera_buffer,
            camera_group,
            opaque,
            transparent,
            meshes,
            format,
            clear: wgpu::Color {
                r: scene.background[0],
                g: scene.background[1],
                b: scene.background[2],
                a: 1.0,
            },
            hud_pipeline,
            hud_group,
            hud_buffer,
            hud_count: 0,
        })
    }

    /// Re-uploads the vertices of the meshes from `first` on (the moving aircraft) and updates their
    /// centres used for sorting glass. The vertex counts must not change.
    pub fn update_meshes(&mut self, first: usize, scene: &Scene) {
        for (gpu, mesh) in self.meshes.iter_mut().zip(&scene.meshes).skip(first) {
            self.queue
                .write_buffer(&gpu.vertex, 0, bytemuck::cast_slice(&mesh.vertices));
            gpu.center = mesh.center;
        }
    }

    /// Sets the screen-space interface drawn over the next frames (an empty slice hides it).
    pub fn set_hud(&mut self, vertices: &[HudVertex]) {
        let count = vertices.len().min(MAX_VERTICES);
        if count > 0 {
            self.queue.write_buffer(
                &self.hud_buffer,
                0,
                bytemuck::cast_slice(&vertices[..count]),
            );
        }
        self.hud_count = count as u32;
    }

    pub fn targets(&self, width: u32, height: u32) -> Targets {
        let depth = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let color = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA color"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Targets {
            depth,
            color,
            width,
            height,
        }
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        targets: &Targets,
        camera: &Camera,
    ) {
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&camera.matrix(targets.width, targets.height).to_cols_array()),
        );
        let mut order: Vec<_> = self.meshes.iter().collect();
        order.sort_by(|a, b| {
            a.glass.cmp(&b.glass).then_with(|| {
                if a.glass {
                    b.center
                        .distance_squared(camera.eye())
                        .total_cmp(&a.center.distance_squared(camera.eye()))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aircraft preview"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.color,
                depth_slice: None,
                resolve_target: Some(view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.camera_group, &[]);
        for mesh in order {
            pass.set_pipeline(if mesh.glass {
                &self.transparent
            } else {
                &self.opaque
            });
            pass.set_bind_group(1, &mesh.material, &[]);
            pass.set_vertex_buffer(0, mesh.vertex.slice(..));
            pass.draw(0..mesh.count, 0..1);
        }
        if self.hud_count > 0 {
            pass.set_pipeline(&self.hud_pipeline);
            pass.set_bind_group(0, &self.hud_group, &[]);
            pass.set_vertex_buffer(0, self.hud_buffer.slice(..));
            pass.draw(0..self.hud_count, 0..1);
        }
    }
}

pub async fn render_png(
    scene: &Scene,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    render_png_camera(scene, &Camera::new(scene), output).await
}

pub async fn render_png_camera(
    scene: &Scene,
    camera: &Camera,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut offscreen = Offscreen::new(scene, 1280, 800).await?;
    offscreen.render(scene, 0, camera, output)
}

/// A renderer that draws to an image instead of a window, reusable for a series of frames: create it once
/// from a scene, then pose the aircraft meshes, move the camera and call `render` for each frame.
pub struct Offscreen {
    renderer: Renderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    targets: Targets,
    width: u32,
    height: u32,
}

impl Offscreen {
    pub async fn new(
        scene: &Scene,
        width: u32,
        height: u32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8UnormSrgb, scene).await?;
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen frame"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let targets = renderer.targets(width, height);
        Ok(Self {
            renderer,
            texture,
            view,
            targets,
            width,
            height,
        })
    }

    /// Sets the interface drawn on the next frames (see `Renderer::set_hud`).
    pub fn set_hud(&mut self, vertices: &[HudVertex]) {
        self.renderer.set_hud(vertices);
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Re-uploads the vertices of the meshes from `first_mesh` on, draws the scene and saves a PNG.
    pub fn render(
        &mut self,
        scene: &Scene,
        first_mesh: usize,
        camera: &Camera,
        output: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (width, height) = (self.width, self.height);
        self.renderer.update_meshes(first_mesh, scene);
        let renderer = &self.renderer;
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        renderer.draw(&mut encoder, &self.view, &self.targets, camera);
        let row_bytes = (width * 4).div_ceil(256) * 256;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row_bytes * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit([encoder.finish()]);
        let (send, recv) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        renderer.device.poll(wgpu::PollType::wait_indefinitely())?;
        recv.recv()??;
        let mapped = buffer.slice(..).get_mapped_range();
        let pixels: Vec<u8> = mapped
            .chunks(row_bytes as usize)
            .flat_map(|row| row[..width as usize * 4].iter().copied())
            .collect();
        image::save_buffer(output, &pixels, width, height, image::ColorType::Rgba8)?;
        drop(mapped);
        buffer.unmap();
        Ok(())
    }
}
