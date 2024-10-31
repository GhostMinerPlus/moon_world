use std::{f32::consts::PI, sync::Arc};

use drawer::{light_mapping::LightMappingBuilder, save_texture, Light};
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
        matrix: drawer::WGPU_OFFSET_M * Matrix4::new_orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 100.0),
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
        let body_v = vec![Arc::new(
            device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(
                    drawer::structs::Body::cube(
                        Matrix4::new_translation(&vector![0.0, 0.0, -5.0])
                            * Matrix4::new_rotation(vector![0.0, PI * 0.25, 0.0]),
                        vector![1.0, 1.0, 1.0, 1.0],
                    )
                    .vertex_v(),
                ),
                usage: BufferUsages::VERTEX,
            }),
        )];

        let (_, depth_tex) = lm_builder.light_mapping(&device, &queue, &light.matrix, &body_v);

        save_texture(
            &device,
            &queue,
            &depth_tex,
            "mapping.png",
            4,
            |c, r, buf_view| {
                let offset = ((r * depth_tex.width() + c) * 4) as usize;

                let depth = f32::from_ne_bytes([
                    buf_view[offset],
                    buf_view[offset + 1],
                    buf_view[offset + 2],
                    buf_view[offset + 3],
                ]);

                let lightness = ((1.0 - depth) * 256.0) as u8;

                image::Rgba([lightness, lightness, lightness, 255])
            },
        );
    });
}

#[test]
fn test() {
    let pos = OFFSET_M
        * Matrix4::new_orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 100.0)
        * vector![0.0, 0.0, -100.0, 1.0];
    println!("{pos}");
}
