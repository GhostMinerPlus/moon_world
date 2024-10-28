use std::{
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use drawer::{structs::Point3Input, Light, ThreeDrawer, ThreeLook};
use image::Rgba;
use nalgebra::{vector, Matrix4};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferDescriptor, BufferUsages, Device, Extent3d, ImageCopyBuffer, ImageDataLayout, Queue,
    Texture, TextureDescriptor, TextureFormat, TextureUsages, TextureViewDescriptor,
};

fn save_texture(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    path: &str,
    f: impl Fn(u32, u32, &[u8]) -> Rgba<u8>,
) {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let (tx, rx) = channel::<bool>();

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
    {
        let buf_view = buffer.slice(..).get_mapped_range();

        let mut img_buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::new(texture.width(), texture.height());

        for (c, r, p) in img_buf.enumerate_pixels_mut() {
            *p = f(c, r, &buf_view);
        }

        let _ = img_buf.save(path);
    }

    buffer.unmap();
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

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
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let look_v = vec![
            ThreeLook::Light(Light {
                color: vector![1.0, 1.0, 1.0, 1.0],
                matrix: Matrix4::identity(),
            }),
            ThreeLook::Body(Arc::new(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[
                    Point3Input {
                        position: [0.0, 0.0, -10.0, 1.0],
                    },
                    Point3Input {
                        position: [1.0, 0.0, -10.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, 1.0, -10.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, 0.0, -5.0, 1.0],
                    },
                    Point3Input {
                        position: [-0.5, 0.0, -5.0, 1.0],
                    },
                    Point3Input {
                        position: [0.0, -0.5, -5.0, 1.0],
                    },
                ]),
                usage: BufferUsages::VERTEX,
            }))),
        ];
        let three_drawer = ThreeDrawer::new(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            Matrix4::new_perspective(1.0, 45.0, 0.1, 500.0),
        );

        let _ = three_drawer.render(
            &device,
            &queue,
            &texture.create_view(&TextureViewDescriptor::default()),
            look_v.iter().collect(),
        );

        save_texture(&device, &queue, &texture, "three.png", |c, r, buf_view| {
            let offset = ((r * texture.width() + c) * 4) as usize;

            image::Rgba([
                buf_view[offset],
                buf_view[offset + 1],
                buf_view[offset + 2],
                buf_view[offset + 3],
            ])
        });
    })
}
