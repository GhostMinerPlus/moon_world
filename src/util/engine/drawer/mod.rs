use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, ComputePassDescriptor, ComputePipeline, Device, Queue,
    RenderPipeline, SurfaceConfiguration, TextureView,
};

use crate::err;

use super::structs::{Line, LineIn, PointInput, Watcher};

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

    pub fn update_line_v(&mut self, device: &Device, line_v: &[Line]) {
        self.line_v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LineV Buffer"),
            contents: &bytemuck::cast_slice(line_v),
            usage: wgpu::BufferUsages::STORAGE,
        });
    }

    pub fn update_watcher(&mut self, device: &Device, watcher: &Watcher) {
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
            &[PointInput::desc()],
            config.format,
            wgpu::PrimitiveTopology::TriangleList,
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
                    PointInput {
                        position: [-1.0, -1.0],
                    },
                    PointInput {
                        position: [1.0, -1.0],
                    },
                    PointInput {
                        position: [1.0, 1.0],
                    },
                    PointInput {
                        position: [-1.0, -1.0],
                    },
                    PointInput {
                        position: [1.0, 1.0],
                    },
                    PointInput {
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
            &[LineIn::desc()],
            config.format,
            wgpu::PrimitiveTopology::LineList,
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
        line_v: &[LineIn],
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

mod pipeline {
    use wgpu::{
        ComputePipeline, ComputePipelineDescriptor, Device, PipelineLayout, RenderPipeline,
        ShaderModule, TextureFormat, VertexBufferLayout,
    };

    pub fn build_render_pipe_line<'a>(
        name: &str,
        device: &Device,
        render_pipeline_layout: &PipelineLayout,
        shader: &ShaderModule,
        buffer_layout_v: &[VertexBufferLayout<'a>],
        format: TextureFormat,
        topology: wgpu::PrimitiveTopology,
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
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
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
