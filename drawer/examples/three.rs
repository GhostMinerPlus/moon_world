use std::{f32::consts::PI, sync::Arc};

use drawer::{save_texture, Body, Light, ThreeDrawer, ThreeLook};
use nalgebra::{vector, Matrix4};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, Extent3d, TextureDescriptor, TextureFormat, TextureUsages, TextureViewDescriptor,
};

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
                view: Matrix4::new_translation(&vector![0.0, 5.0, 0.0])
                    * Matrix4::new_rotation(vector![PI * 0.25, 0.0, 0.0]),
                proj: drawer::WGPU_OFFSET_M
                    * Matrix4::new_orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 20.0),
            }),
            ThreeLook::Body(Body {
                model_m: Matrix4::new_translation(&vector![0.0, 0.0, -3.0])
                    * Matrix4::new_rotation(vector![0.0, -PI * 0.25, 0.0]),
                buf: Arc::new(
                    device.create_buffer_init(&BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(
                            drawer::structs::Point3InputArray::cube(vector![1.0, 1.0, 1.0, 1.0])
                                .vertex_v(),
                        ),
                        usage: BufferUsages::VERTEX,
                    }),
                ),
            }),
            ThreeLook::Body(Body {
                model_m: Matrix4::new_translation(&vector![0.0, 1.0, -3.0])
                    * Matrix4::new_rotation(vector![0.0, -PI * 0.45, 0.0]),
                buf: Arc::new(
                    device.create_buffer_init(&BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(
                            drawer::structs::Point3InputArray::cube(vector![1.0, 1.0, 1.0, 1.0])
                                .vertex_v(),
                        ),
                        usage: BufferUsages::VERTEX,
                    }),
                ),
            }),
        ];
        let three_drawer = ThreeDrawer::new(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            drawer::WGPU_OFFSET_M * Matrix4::new_perspective(1.0, PI * 0.6, 0.1, 500.0),
        );

        let _ = three_drawer.render(
            &device,
            &queue,
            &texture.create_view(&TextureViewDescriptor::default()),
            look_v.iter().collect(),
        );

        save_texture(
            &device,
            &queue,
            &texture,
            "three.png",
            4,
            |c, r, buf_view| {
                let offset = ((r * texture.width() + c) * 4) as usize;

                image::Rgba([
                    buf_view[offset],
                    buf_view[offset + 1],
                    buf_view[offset + 2],
                    buf_view[offset + 3],
                ])
            },
        );
    })
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use nalgebra::{vector, Matrix4, Vector4};

    fn reverse_vec_from_mat(v: Vector4<f32>, m: Matrix4<f32>) -> Vector4<f32> {
        let ox = m * vector![1.0, 0.0, 0.0, 0.0];
        let oy = m * vector![0.0, 1.0, 0.0, 0.0];
        let oz = m * vector![0.0, 0.0, 1.0, 0.0];

        return vector![v.dot(&ox), v.dot(&oy), v.dot(&oz), 0.0];
    }

    #[test]
    fn test() {
        let m = Matrix4::new_translation(&vector![0.0, 10.0, 10.0])
            * Matrix4::new_rotation(vector![0.0, PI * 0.25, 0.0]);
        let v = vector![0.0, 0.0, -1.0, 0.0];

        println!("{}", reverse_vec_from_mat(v, m).normalize());
    }
}
