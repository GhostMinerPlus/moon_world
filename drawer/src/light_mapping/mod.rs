use std::sync::Arc;

use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, Buffer, BufferUsages, Device, Extent3d, Queue, RenderPipeline, Texture,
    TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::structs::Point3Input;

use super::pipeline;

pub struct LightMappingBuilder {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    format: TextureFormat,
}

impl LightMappingBuilder {
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
            "Light Mapping Pipeline",
            &device,
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Mapping Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Light Mapping Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/light_mapping.wgsl").into()),
            }),
            &[Point3Input::pos_only_desc()],
            format,
            wgpu::PrimitiveTopology::TriangleList,
            None,
        );

        Self {
            render_pipeline,
            bind_group_layout,
            format,
        }
    }

    pub fn light_mapping(
        &self,
        device: &Device,
        queue: &Queue,
        light: &Matrix4<f32>,
        body_v: &[Arc<Buffer>],
    ) -> Texture {
        let light_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(light.as_slice()),
            usage: BufferUsages::UNIFORM,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: light_buf.as_entire_binding(),
                    }],
                    label: Some("bind_group0"),
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

        texture
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::channel, time::Duration};

    use nalgebra::{vector, Matrix4};
    use wgpu::{BufferDescriptor, ImageCopyBuffer, ImageDataLayout};

    use crate::Light;

    use super::*;

    #[test]
    fn test() {
        let _ =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
                .is_test(true)
                .try_init();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let light = Light {
            color: vector![1.0, 1.0, 1.0, 1.0],
            matrix: Matrix4::identity(),
        };
        let (tx, rx) = channel::<bool>();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

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

            let lm_builder = LightMappingBuilder::new(&device, TextureFormat::Rgba8Unorm);
            let body_v = vec![Arc::new(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    Point3Input {
                        position: [0.0, 0.0, 0.5, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                        normal: [0.0, 0.0, 1.0, 0.0]
                    },
                    Point3Input {
                        position: [1.0, 0.0, 0.5, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                        normal: [0.0, 0.0, 1.0, 0.0]
                    },
                    Point3Input {
                        position: [0.0, 1.0, 0.5, 1.0],
                        color: [1.0, 1.0, 1.0, 1.0],
                        normal: [0.0, 0.0, 1.0, 0.0]
                    },
                ]),
                usage: BufferUsages::VERTEX,
            }))];
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let texture = lm_builder.light_mapping(&device, &queue, &light.matrix, &body_v);

            let buffer = device.create_buffer(&BufferDescriptor {
                label: None,
                size: (texture.width() * texture.height() * 4) as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_texture_to_buffer(
                texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(texture.width() * 4),
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

            let buf_view = buffer.slice(..).get_mapped_range();

            let b = buf_view[(511 * texture.width() as usize + 512) * 4];

            drop(buf_view);

            buffer.unmap();

            assert_ne!(b, 0);
        });
    }
}
