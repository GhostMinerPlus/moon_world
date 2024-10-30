use std::sync::Arc;

use drawer::{light_mapping::LightMappingBuilder, save_texture, structs::Point3Input, Light};
use nalgebra::{vector, Matrix4};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, TextureFormat,
};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let light = Light {
        color: vector![1.0, 1.0, 1.0, 1.0],
        matrix: Matrix4::identity(),
    };
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
                Point3Input {
                    position: [0.0, 0.0, 0.2, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0]
                },
                Point3Input {
                    position: [0.5, 0.0, 0.2, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0]
                },
                Point3Input {
                    position: [0.0, 0.5, 0.2, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    normal: [0.0, 0.0, 1.0, 0.0]
                },
            ]),
            usage: BufferUsages::VERTEX,
        }))];

        let texture = lm_builder.light_mapping(&device, &queue, &light.matrix, &body_v);

        save_texture(
            &device,
            &queue,
            &texture,
            "mapping.png",
            |c, r, buf_view| {
                let offset = ((r * texture.width() + c) * 4) as usize;

                let depth = u32::from_be_bytes([
                    buf_view[offset],
                    buf_view[offset + 1],
                    buf_view[offset + 2],
                    buf_view[offset + 3],
                ]) as f32
                    / (256.0 * 256.0 * 256.0 * 256.0);

                let lightness = if depth <= 0.0 {
                    0
                } else {
                    ((1.0 - depth) * 256.0) as u8
                };

                image::Rgba([lightness, lightness, lightness, lightness])
            },
        );
    });
}
