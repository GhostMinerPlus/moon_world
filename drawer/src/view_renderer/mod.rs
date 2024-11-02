use std::sync::Arc;

use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, Color, DepthBiasState, DepthStencilState, Device,
    Extent3d, Operations, Queue, RenderPassDepthStencilAttachment, RenderPipeline, StencilState,
    Texture, TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::{pipeline, structs::Point3Input};

pub struct ViewRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    view_texture: Texture,
    depth_texture: Texture,
}

impl ViewRenderer {
    pub fn new(device: &Device) -> Self {
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
            TextureFormat::Rgba32Float,
            wgpu::PrimitiveTopology::TriangleList,
            Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
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
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_texture = device.create_texture(&TextureDescriptor {
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
            #[cfg(test)]
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC,
            #[cfg(not(test))]
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self {
            render_pipeline,
            bind_group_layout,
            view_texture,
            depth_texture,
        }
    }

    pub fn view_renderer(
        &self,
        device: &Device,
        queue: &Queue,
        mv: &Matrix4<f32>,
        proj: &Matrix4<f32>,
        body_v: &[Arc<Buffer>],
    ) -> &Texture {
        let mv_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(mv.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let proj_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(proj.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let view_texture_view = self
                .view_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let depth_texture_view = self
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth_texture_view,
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
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: mv_buf.as_entire_binding(),
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

        &self.view_texture
    }
}

#[cfg(test)]
mod tests {
    use std::{f32::consts::PI, sync::Arc};

    use crate::{structs, WGPU_OFFSET_M};
    use nalgebra::{vector, Matrix4};
    use wgpu::{
        util::{BufferInitDescriptor, DeviceExt},
        BufferUsages,
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
                        required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                            | wgpu::Features::VERTEX_WRITABLE_STORAGE
                            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
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

            let renderer = ViewRenderer::new(&device);
            let look_v = vec![Arc::new(
                device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(
                        structs::Body::cube(
                            Matrix4::new_translation(&vector![0.0, 0.0, -2.0])
                                * Matrix4::new_rotation(vector![0.0, PI * 0.25, 0.0]),
                            vector![1.0, 1.0, 1.0, 1.0],
                        )
                        .vertex_v(),
                    ),
                    usage: BufferUsages::VERTEX,
                }),
            )];

            renderer.view_renderer(
                &device,
                &queue,
                &Matrix4::identity(),
                &(WGPU_OFFSET_M * Matrix4::new_perspective(1.0, PI * 0.6, 0.1, 500.0)),
                &look_v,
            );
        })
    }

    #[test]
    fn test_perspective() {
        let proj = WGPU_OFFSET_M * Matrix4::new_perspective(1.0, PI * 0.6, 0.1, 500.0);

        let v_pos = proj * vector![1.0, 1.0, -3.0, 1.0];

        println!("{}", v_pos);
    }

    #[test]
    fn test_orthographic() {
        let proj = WGPU_OFFSET_M * Matrix4::new_orthographic(-1.0, 1.0, -1.0, 1.0, 0.1, 500.0);

        let v_pos = proj * vector![0.0, 0.0, -400.0, 1.0];

        println!("{}", v_pos);
    }
}
