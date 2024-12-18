use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, BlendState, BufferUsages, Device, Queue, RenderPipeline, Texture,
    TextureFormat, TextureView, TextureViewDescriptor,
};

use crate::{err, pipeline, structs::Point3Input, Light};

mod inner {
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BindGroupLayout, Buffer, BufferUsages, Device, RenderPass, TextureView,
    };

    use crate::structs::Point3Input;

    pub fn render_light(
        render_pass: &mut RenderPass,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        view_buf: &Buffer,
        proj_buf: &Buffer,
        light_v_buf: &Buffer,
        light_p_buf: &Buffer,
        view_texture: &TextureView,
        light_texture: &TextureView,
        light_depth_tex: &TextureView,
        ratio: f32,
    ) {
        let body = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[
                Point3Input {
                    position: [-1.0, 1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Point3Input {
                    position: [-1.0, -1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Point3Input {
                    position: [1.0, -1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Point3Input {
                    position: [-1.0, 1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Point3Input {
                    position: [1.0, -1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
                Point3Input {
                    position: [1.0, 1.0, 0.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0],
                },
            ]),
            usage: BufferUsages::VERTEX,
        });

        let ratio_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: &ratio.to_ne_bytes(),
            usage: BufferUsages::UNIFORM,
        });

        render_pass.set_bind_group(
            0,
            &device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: view_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: proj_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: light_v_buf.as_entire_binding(),
                    },
                    // view_tex
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(view_texture),
                    },
                    // light_tex
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(light_texture),
                    },
                    // light_depth_tex
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(light_depth_tex),
                    },
                    // light_p
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: light_p_buf.as_entire_binding(),
                    },
                    // ratio
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: ratio_buf.as_entire_binding(),
                    },
                ],
                label: None,
            }),
            &[],
        );

        render_pass.set_vertex_buffer(0, body.slice(..));

        render_pass.draw(0..6, 0..1);
    }
}

pub struct BodyRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl BodyRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // view
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // proj
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // light_v
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // view_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // light_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // light_depth_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // light_p
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // ratio
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("light"),
        });

        let render_pipeline = pipeline::RenderPipelineBuilder::new(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/body_render.wgsl").into()),
            }),
            &[Point3Input::pos_only_desc()],
            format,
        )
        .set_name(Some("Body Render Pipeline"))
        .set_blend(Some(BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::DstAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::DstAlpha,
                operation: wgpu::BlendOperation::Max,
            },
        }))
        .build(device);

        Self {
            render_pipeline,
            bind_group_layout,
        }
    }

    /// called => body = rendered
    pub fn body_render(
        &self,
        device: &Device,
        queue: &Queue,
        surface: &TextureView,
        view_texture: &Texture,
        light_texture_v: Vec<(&Light, (Texture, Texture))>,
        view_m: &Matrix4<f32>,
        proj_m: &Matrix4<f32>,
        ratio: f32,
    ) -> err::Result<()> {
        let view_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(view_m.data.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let proj_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(proj_m.data.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        let light_texture_view_v = light_texture_v
            .iter()
            .map(|(light, (color_tex, depth_tex))| {
                (
                    (&light.view, &light.proj),
                    (
                        color_tex.create_view(&TextureViewDescriptor::default()),
                        depth_tex.create_view(&TextureViewDescriptor::default()),
                    ),
                )
            })
            .collect::<Vec<((&Matrix4<f32>, &Matrix4<f32>), (TextureView, TextureView))>>();
        let view_texture_view = view_texture.create_view(&TextureViewDescriptor::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface,
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

            render_pass.set_pipeline(&self.render_pipeline);

            for ((light_v, light_p), (color_texture_view, depth_tex_view)) in &light_texture_view_v
            {
                let light_v_buf = device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(light_v.data.as_slice()),
                    usage: BufferUsages::UNIFORM,
                });
                let light_p_buf = device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(light_p.data.as_slice()),
                    usage: BufferUsages::UNIFORM,
                });

                inner::render_light(
                    &mut render_pass,
                    device,
                    &self.bind_group_layout,
                    &view_buf,
                    &proj_buf,
                    &light_v_buf,
                    &light_p_buf,
                    &view_texture_view,
                    color_texture_view,
                    depth_tex_view,
                    ratio,
                );
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
