use nalgebra::Matrix4;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupLayout, BufferUsages, Color, DepthBiasState, DepthStencilState, Device, Extent3d,
    Queue, RenderPassDepthStencilAttachment, RenderPipeline, StencilState, Texture,
    TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::{structs::Point3Input, Body};

use super::pipeline;

pub struct LightMappingBuilder {
    render_pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl LightMappingBuilder {
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

        let render_pipeline = pipeline::RenderPipelineBuilder::new(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Mapping Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
            &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Light Mapping Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader/light_mapping.wgsl").into()),
            }),
            &[Point3Input::desc()],
            TextureFormat::Rgba32Float,
        )
        .set_name(Some("Light Mapping Pipeline"))
        .set_depth_stencil(Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }))
        .build(&device);

        Self {
            render_pipeline,
            bind_group_layout,
        }
    }

    pub fn light_mapping(
        &self,
        device: &Device,
        queue: &Queue,
        light: &Matrix4<f32>,
        body_v: &[&Body],
    ) -> (Texture, Texture) {
        let light_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(light.as_slice()),
            usage: BufferUsages::UNIFORM,
        });

        let color_texture = device.create_texture(&TextureDescriptor {
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
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
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
            #[cfg(not(test))]
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            #[cfg(test)]
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut is_first = true;

        for body in body_v {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
            let model_buf = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(body.model_m.as_slice()),
                usage: BufferUsages::UNIFORM,
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if is_first {
                                wgpu::LoadOp::Clear(Color::TRANSPARENT)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: if is_first {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
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
                                resource: light_buf.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: model_buf.as_entire_binding(),
                            },
                        ],
                        label: Some("bind_group0"),
                    }),
                    &[],
                );

                render_pass.set_vertex_buffer(0, body.buf.slice(..));
                render_pass.draw(
                    0..(body.buf.size() as usize / std::mem::size_of::<Point3Input>()) as u32,
                    0..1,
                );
            }

            queue.submit(std::iter::once(encoder.finish()));

            is_first = false;
        }

        (color_texture, depth_texture)
    }
}

#[cfg(test)]
mod tests {
    use std::{f32::consts::PI, sync::Arc};

    use nalgebra::{vector, Matrix4};

    use crate::{save_texture, structs, Light, WGPU_OFFSET_M};

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

        rt.block_on(async move {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            let light = Light {
                color: vector![1.0, 1.0, 1.0, 1.0],
                view: Matrix4::new_translation(&vector![0.0, 2.5, 0.0])
                    * Matrix4::new_rotation(vector![PI * 0.25, 0.0, 0.0]),
                proj: WGPU_OFFSET_M
                    * Matrix4::new_orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 500.0),
            };

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

            let lm_builder = LightMappingBuilder::new(&device);
            let body_v = vec![Body {
                model_m: Matrix4::new_translation(&vector![0.0, 0.0, -3.0])
                    * Matrix4::new_rotation(vector![0.0, -PI * 0.25, 0.0]),
                buf: Arc::new(device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(
                        structs::Point3InputArray::cube(vector![1.0, 1.0, 1.0, 1.0]).vertex_v(),
                    ),
                    usage: BufferUsages::VERTEX,
                })),
            }];

            let (_, depth_texture) = lm_builder.light_mapping(
                &device,
                &queue,
                &(light.proj * light.view),
                &body_v.iter().collect::<Vec<&Body>>(),
            );

            save_texture(
                &device,
                &queue,
                &depth_texture,
                "light_depth.png",
                4,
                |c, r, buf| {
                    let offset = ((r * depth_texture.width() + c) * 4) as usize;

                    let depth = f32::from_ne_bytes([
                        buf[offset],
                        buf[offset + 1],
                        buf[offset + 2],
                        buf[offset + 3],
                    ]);

                    let lightness = ((1.0 - depth) * 256.0) as u8;

                    image::Rgba([lightness, lightness, lightness, 255])
                },
            );
        });
    }
}
