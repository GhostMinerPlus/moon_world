use std::sync::Arc;

use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, Device, Queue, RenderPipeline, Texture, TextureFormat,
    TextureView,
};

use crate::{err, pipeline, structs::Point3Input, Light};

pub struct BodyRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    format: TextureFormat,
}

impl BodyRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
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
            ],
            label: Some("light"),
        });

        let render_pipeline = pipeline::build_render_pipe_line(
            "Light Mapping Pipeline",
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
            &[Point3Input::desc()],
            format,
            wgpu::PrimitiveTopology::TriangleList,
        );

        Self {
            render_pipeline,
            bind_group_layout,
            format,
        }
    }

    /// called => body = rendered
    pub fn body_render(
        &self,
        device: &Device,
        queue: &Queue,
        view: &TextureView,
        light_texture_v: Vec<(&Light, Texture)>,
        body_v: &Vec<Arc<Buffer>>,
        view_m: &Matrix4<f32>,
        proj_m: &Matrix4<f32>,
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(
                0,
                &device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: view_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: proj_buf.as_entire_binding(),
                        },
                    ],
                    label: None,
                }),
                &[],
            );

            for body in body_v {
                render_pass.set_vertex_buffer(0, body.slice(..));
                render_pass.draw(
                    0..(body.size() as usize / std::mem::size_of::<Point3Input>()) as u32,
                    0..1,
                );
            }
        }

        queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}
