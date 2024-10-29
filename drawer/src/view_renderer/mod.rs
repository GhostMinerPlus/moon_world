use std::sync::Arc;

use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, DepthBiasState, DepthStencilState, Device, Extent3d,
    Operations, Queue, RenderPassDepthStencilAttachment, RenderPipeline, StencilState, Texture,
    TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::{pipeline, structs::Point3Input};

pub struct ViewRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    view_texture: Texture,
    view_depth_texture: Texture,
}

impl ViewRenderer {
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light"),
        });

        let render_pipeline = pipeline::build_render_pipe_line(
            "View Render Pipeline",
            &device,
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("View Render Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("View Render Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/view_renderer.wgsl").into()),
            }),
            &[Point3Input::desc()],
            format,
            wgpu::PrimitiveTopology::TriangleList,
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
        );
        let view_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view_depth_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self {
            render_pipeline,
            bind_group_layout,
            view_texture,
            view_depth_texture,
        }
    }

    pub fn view_renderer(
        &self,
        device: &Device,
        queue: &Queue,
        mvp: &Matrix4<f32>,
        body_v: &[Arc<Buffer>],
    ) -> (&Texture, &Texture) {
        let mvp_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(mvp.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let view_texture_view = self
                .view_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let view_depth_texture_view = self
                .view_depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &view_depth_texture_view,
                    depth_ops: Some(Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(
                0,
                &device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: mvp_buf.as_entire_binding(),
                    }],
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

        (&self.view_texture, &self.view_depth_texture)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::structs::Point3Input;
    use nalgebra::Matrix4;
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BufferUsages, TextureFormat,
    };

    use super::*;

    #[test]
    fn test() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        rt.block_on(async move {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                        // WebGL doesn't support all of wgpu's features, so if
                        // we're building for the web we'll have to disable some.
                        required_limits: wgpu::Limits::default(),
                        label: None,
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None, // Trace path
                )
                .await
                .unwrap();

            let renderer = ViewRenderer::new(&device, TextureFormat::Rgba8Unorm);
            let look_v = vec![Arc::new(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    Point3Input {
                        position: [0.0, 0.0, -10.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    Point3Input {
                        position: [1.0, 0.0, -10.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, 1.0, -10.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, 0.0, -5.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    Point3Input {
                        position: [-0.5, 0.0, -5.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, -0.5, -5.0, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                    },
                ]),
                usage: BufferUsages::VERTEX,
            }))];

            renderer.view_renderer(
                &device,
                &queue,
                &Matrix4::new_perspective(1.0, 45.0, 0.1, 500.0),
                &look_v,
            );
        })
    }
}
