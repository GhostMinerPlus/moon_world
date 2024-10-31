use std::sync::Arc;

use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, Device, Extent3d, Queue,
    RenderPipeline, SamplerDescriptor, Texture, TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::{pipeline, structs::Point3Input};

pub struct ViewRenderer {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    view_texture: Texture,
    view_normal_texture: Texture,
}

impl ViewRenderer {
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
                // normal_tex
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
            format,
            wgpu::PrimitiveTopology::TriangleList,
            None,
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
        let view_normal_texture = device.create_texture(&TextureDescriptor {
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
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        Self {
            render_pipeline,
            bind_group_layout,
            view_texture,
            view_normal_texture,
        }
    }

    pub fn view_renderer(
        &self,
        device: &Device,
        queue: &Queue,
        mv: &Matrix4<f32>,
        proj: &Matrix4<f32>,
        body_v: &[Arc<Buffer>],
    ) -> (&Texture, Texture, &Texture) {
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
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
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
            let view_depth_texture_view =
                view_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let view_normal_texture_view = self
                .view_normal_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
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
                            resource: mv_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: proj_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&view_normal_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&view_depth_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&sampler),
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

        (
            &self.view_texture,
            view_depth_texture,
            &self.view_normal_texture,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::structs;
    use nalgebra::{vector, Matrix4};
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

            let renderer = ViewRenderer::new(&device, TextureFormat::Rgba8Unorm);
            let look_v = vec![Arc::new(
                device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(
                        &structs::Body::cube(
                            Matrix4::new_translation(&vector![0.0, 0.0, -5.0])
                                * Matrix4::new_rotation(vector![0.0, 1.0, 0.0]),
                            vector![1.0, 1.0, 1.0, 1.0],
                        )
                        .vertex_v()[0..24],
                    ),
                    usage: BufferUsages::VERTEX,
                }),
            )];

            renderer.view_renderer(
                &device,
                &queue,
                &Matrix4::identity(),
                &Matrix4::new_perspective(1.0, 120.0, 0.1, 500.0),
                &look_v,
            );
        })
    }
}
