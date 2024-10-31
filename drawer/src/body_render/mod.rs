use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, BufferUsages, Device, Queue, RenderPipeline, Texture, TextureFormat,
    TextureView, TextureViewDescriptor,
};

use crate::{err, pipeline, save_texture, structs::Point3Input, Light};

mod inner {
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BindGroupLayout, Buffer, BufferUsages, Device, RenderPass, SamplerDescriptor, TextureView,
    };

    use crate::structs::Point3Input;

    pub fn render_light(
        render_pass: &mut RenderPass,
        device: &Device,
        bind_group_layout: &BindGroupLayout,
        view_buf: &Buffer,
        proj_buf: &Buffer,
        light_buf: &Buffer,
        view_texture: &TextureView,
        view_depth_texture: &TextureView,
        view_normal_texture: &TextureView,
        light_texture: &TextureView,
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
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
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
                        resource: light_buf.as_entire_binding(),
                    },
                    // view_tex
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(view_texture),
                    },
                    // view_depth_tex
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(view_depth_texture),
                    },
                    //view_normal_tex
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(view_normal_texture),
                    },
                    // light_tex
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(light_texture),
                    },
                    // sampler
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::Sampler(&sampler),
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
                // light
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // view_depth_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // view_normal_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
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
                    binding: 6,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("light"),
        });

        let render_pipeline = pipeline::build_render_pipe_line(
            "Body Render Pipeline",
            &device,
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
            wgpu::PrimitiveTopology::TriangleList,
            None,
        );

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
        view_depth_texture: &Texture,
        view_noraml_texture: &Texture,
        light_texture_v: Vec<(&Light, Texture)>,
        view_m: &Matrix4<f32>,
        proj_m: &Matrix4<f32>,
    ) -> err::Result<()> {
        save_texture(
            device,
            queue,
            &view_noraml_texture,
            "normal.png",
            16,
            |c, r, buf_view| {
                let offset = ((r * view_noraml_texture.width() + c) * 16) as usize;

                let r = f32::from_ne_bytes([
                    buf_view[offset],
                    buf_view[offset + 1],
                    buf_view[offset + 2],
                    buf_view[offset + 3],
                ]) * 256.0;
                let g = f32::from_ne_bytes([
                    buf_view[offset + 4],
                    buf_view[offset + 5],
                    buf_view[offset + 6],
                    buf_view[offset + 7],
                ]) * 256.0;
                let b = f32::from_ne_bytes([
                    buf_view[offset + 8],
                    buf_view[offset + 9],
                    buf_view[offset + 10],
                    buf_view[offset + 11],
                ]) * 256.0;

                image::Rgba([r as u8, g as u8, b as u8, 255])
            },
        );

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
            .map(|(light, tex)| {
                (
                    &light.matrix,
                    tex.create_view(&TextureViewDescriptor::default()),
                )
            })
            .collect::<Vec<(&Matrix4<f32>, TextureView)>>();
        let view_depth_texture_view =
            view_depth_texture.create_view(&TextureViewDescriptor::default());
        let view_texture_view = view_texture.create_view(&TextureViewDescriptor::default());
        let view_noraml_texture_view =
            view_noraml_texture.create_view(&TextureViewDescriptor::default());

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

            for (light_m, light_texture_view) in &light_texture_view_v {
                let light_buf = device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(light_m.data.as_slice()),
                    usage: BufferUsages::UNIFORM,
                });

                inner::render_light(
                    &mut render_pass,
                    device,
                    &self.bind_group_layout,
                    &view_buf,
                    &proj_buf,
                    &light_buf,
                    &view_texture_view,
                    &view_depth_texture_view,
                    &view_noraml_texture_view,
                    light_texture_view,
                );
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
