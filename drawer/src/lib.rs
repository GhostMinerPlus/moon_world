use std::{
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use image::Rgba;
use nalgebra::{Matrix4, Vector4};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferDescriptor, BufferUsages, ComputePassDescriptor,
    ComputePipeline, Device, ImageCopyBuffer, ImageDataLayout, Queue, RenderPipeline,
    SurfaceConfiguration, Texture, TextureFormat, TextureView,
};

mod pipeline {
    use wgpu::{
        ComputePipeline, ComputePipelineDescriptor, DepthStencilState, Device, PipelineLayout,
        RenderPipeline, ShaderModule, TextureFormat, VertexBufferLayout,
    };

    pub fn build_render_pipe_line<'a>(
        name: &str,
        device: &Device,
        render_pipeline_layout: &PipelineLayout,
        shader: &ShaderModule,
        buffer_layout_v: &[VertexBufferLayout<'a>],
        format: TextureFormat,
        topology: wgpu::PrimitiveTopology,
        depth_stencil_op: Option<DepthStencilState>,
    ) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: buffer_layout_v,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                ..Default::default()
            },
            depth_stencil: depth_stencil_op,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    pub fn build_compute_pipeline(
        name: &str,
        device: &Device,
        pipeline_layout: &PipelineLayout,
        shader: &ShaderModule,
        entry_point: &str,
    ) -> ComputePipeline {
        device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(pipeline_layout),
            module: shader,
            entry_point,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }
}
mod body_render;
mod view_renderer;

pub mod err;
pub mod light_mapping;
pub mod structs;

pub enum ThreeLook {
    Body(Arc<wgpu::Buffer>),
    Light(Light),
}

pub struct Light {
    pub color: Vector4<f32>,
    pub matrix: Matrix4<f32>,
}

pub struct RayDrawer {
    compute_bind_group_layout: BindGroupLayout,
    compute_pipeline: ComputePipeline,
    compute_texture_buffer: Buffer,
    size_buffer: Buffer,
    line_v_buffer: Buffer,
    watcher_buffer: Buffer,
}

impl RayDrawer {
    pub fn new(device: &Device, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/compute.wgsl").into()),
        });
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("compute_bind_group_layout"),
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = pipeline::build_compute_pipeline(
            "Compute Pipeline",
            &device,
            &pipeline_layout,
            &compute_shader,
            "main",
        );

        let sz = (size.width * size.height * 4) as usize;
        let mut data = Vec::with_capacity(sz);
        data.resize(sz, 0);
        let compute_texture_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Buffer"),
            contents: &data,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST,
        });
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ratio Buffer"),
            contents: bytemuck::cast_slice(&[size.width, size.height]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let line_v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LineV Buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::STORAGE,
        });
        let watcher_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Watcher Buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self {
            compute_bind_group_layout,
            compute_pipeline,
            compute_texture_buffer,
            size_buffer,
            line_v_buffer,
            watcher_buffer,
        }
    }

    pub fn draw_ray_to_point_texture(&self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });
        {
            encoder.clear_buffer(&self.compute_texture_buffer, 0, None);
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.compute_texture_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.size_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.line_v_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: self.watcher_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("compute_texture_bind_group"),
                }),
                &[],
            );
            compute_pass.dispatch_workgroups(20, 1, 1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn get_result_buffer(&self) -> &Buffer {
        &self.compute_texture_buffer
    }

    pub fn get_size_buffer(&self) -> &Buffer {
        &self.size_buffer
    }

    pub fn resize(&mut self, device: &Device, queue: &Queue, size: winit::dpi::PhysicalSize<u32>) {
        queue.write_buffer(
            &self.size_buffer,
            0,
            bytemuck::cast_slice(&[size.width, size.height]),
        );
        let sz = (size.width * size.height * 4) as usize;
        let mut data = Vec::with_capacity(sz);
        data.resize(sz, 0);
        let sz = (size.width * size.height * 4) as usize;
        let mut data = Vec::with_capacity(sz);
        data.resize(sz, 0);
        self.compute_texture_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Texture Buffer"),
                contents: &data,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
            });
    }

    pub fn update_line_v(&mut self, device: &Device, line_v: &[structs::Line]) {
        self.line_v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LineV Buffer"),
            contents: &bytemuck::cast_slice(line_v),
            usage: wgpu::BufferUsages::STORAGE,
        });
    }

    pub fn update_watcher(&mut self, device: &Device, watcher: &structs::Watcher) {
        self.watcher_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Watcher Buffer"),
            contents: &bytemuck::cast_slice(&[*watcher]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
    }

    pub fn get_watcher_buffer(&self) -> &Buffer {
        &self.watcher_buffer
    }
}

pub struct SurfaceDrawer {
    triangle_render_pipeline: RenderPipeline,
    texture_bind_group_layout: BindGroupLayout,
}

impl SurfaceDrawer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let triangle_render_pipeline = pipeline::build_render_pipe_line(
            "Point Pipeline",
            &device,
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Point Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Point Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/point.wgsl").into()),
            }),
            &[structs::PointInput::desc()],
            config.format,
            wgpu::PrimitiveTopology::TriangleList,
            None,
        );
        Self {
            triangle_render_pipeline,
            texture_bind_group_layout,
        }
    }

    pub fn draw_point_to_surface<'a>(
        &self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        compute_texture_buffer: &Buffer,
        size_buffer: &Buffer,
    ) -> err::Result<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.triangle_render_pipeline);
            render_pass.set_bind_group(
                0,
                &device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: compute_texture_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: size_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("texture_bind_group"),
                }),
                &[],
            );
            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    structs::PointInput {
                        position: [-1.0, -1.0],
                    },
                    structs::PointInput {
                        position: [1.0, -1.0],
                    },
                    structs::PointInput {
                        position: [1.0, 1.0],
                    },
                    structs::PointInput {
                        position: [-1.0, -1.0],
                    },
                    structs::PointInput {
                        position: [1.0, 1.0],
                    },
                    structs::PointInput {
                        position: [-1.0, 1.0],
                    },
                ]),
                usage: BufferUsages::VERTEX,
            });
            render_pass.set_vertex_buffer(0, buffer.slice(..));
            render_pass.draw(0..6, 0..1);
            // denoise
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

pub struct WathcerDrawer {
    line_render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl WathcerDrawer {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("bind_group0_layout"),
        });

        let line_render_pipeline = pipeline::build_render_pipe_line(
            "Line Pipeline",
            &device,
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Line Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/line.wgsl").into()),
            }),
            &[structs::LineIn::desc()],
            config.format,
            wgpu::PrimitiveTopology::LineList,
            None,
        );

        Self {
            line_render_pipeline,
            bind_group_layout,
        }
    }

    ///
    pub fn draw_light_to_surface<'a>(
        &self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        watcher_buffer: &Buffer,
        size_buffer: &Buffer,
        line_v: &[structs::LineIn],
    ) -> err::Result<()> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.line_render_pipeline);
            render_pass.set_bind_group(
                0,
                &device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: watcher_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: size_buffer.as_entire_binding(),
                        },
                    ],
                    label: Some("bind_group0"),
                }),
                &[],
            );

            let buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(line_v),
                usage: BufferUsages::VERTEX,
            });
            render_pass.set_vertex_buffer(0, buffer.slice(..));
            render_pass.draw(0..line_v.len() as u32, 0..1);
            // denoise
        }
        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

pub struct ThreeDrawer {
    light_mapping_builder: light_mapping::LightMappingBuilder,
    body_renderer: body_render::BodyRenderer,
    view_m: Matrix4<f32>,
    proj_m: Matrix4<f32>,
    view_renderer: view_renderer::ViewRenderer,
}

impl ThreeDrawer {
    pub fn new(device: &Device, format: TextureFormat, proj_m: Matrix4<f32>) -> Self {
        let light_mapping_builder = light_mapping::LightMappingBuilder::new(device, format);
        let body_renderer = body_render::BodyRenderer::new(device, format);
        let view_renderer = view_renderer::ViewRenderer::new(device, format);

        Self {
            light_mapping_builder,
            body_renderer,
            view_m: Matrix4::identity(),
            proj_m,
            view_renderer,
        }
    }

    pub fn render(
        &self,
        device: &Device,
        queue: &Queue,
        surface: &TextureView,
        look_v: Vec<&ThreeLook>,
    ) -> err::Result<()> {
        let mut body_buffer_v = vec![];
        let mut light_v = vec![];

        for look in look_v {
            match look {
                ThreeLook::Body(buffer) => body_buffer_v.push(buffer.clone()),
                ThreeLook::Light(light) => light_v.push(light),
            }
        }

        // mapping of light_v
        let light_texture_v = light_v
            .iter()
            .map(|light| {
                (
                    *light,
                    self.light_mapping_builder.light_mapping(
                        device,
                        queue,
                        &light.matrix,
                        &body_buffer_v,
                    ),
                )
            })
            .collect::<Vec<(&Light, Texture)>>();
        // color and depth of view
        let (view_texture, view_depth_texture, view_normal_texture) = self
            .view_renderer
            .view_renderer(device, queue, &self.view_m, &self.proj_m, &body_buffer_v);

        self.body_renderer.body_render(
            device,
            queue,
            surface,
            view_texture,
            &view_depth_texture,
            view_normal_texture,
            light_texture_v,
            &self.view_m,
            &self.proj_m,
        )
    }
}

pub fn save_texture(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    path: &str,
    p_sz: usize,
    f: impl Fn(u32, u32, &[u8]) -> Rgba<u8>,
) {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let (tx, rx) = channel::<bool>();

    let buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: (texture.width() * texture.height() * p_sz as u32) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * p_sz as u32),
                rows_per_image: None,
            },
        },
        texture.size(),
    );

    queue.submit(std::iter::once(encoder.finish()));

    buffer.slice(..).map_async(wgpu::MapMode::Read, move |rs| {
        if let Err(e) = rs {
            log::error!("{e:?}");
            let _ = tx.send(false);
        } else {
            let _ = tx.send(true);
        }
    });

    device.poll(wgpu::MaintainBase::Wait).panic_on_timeout();

    if !rx.recv_timeout(Duration::from_secs(3)).unwrap() {
        panic!("texture data is invalid!");
    }

    log::info!("mapped");
    {
        let buf_view = buffer.slice(..).get_mapped_range();

        let mut img_buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::new(texture.width(), texture.height());

        for (c, r, p) in img_buf.enumerate_pixels_mut() {
            *p = f(c, r, &buf_view);
        }

        let _ = img_buf.save(path);
    }

    buffer.unmap();
}
