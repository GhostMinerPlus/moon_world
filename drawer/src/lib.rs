use std::{
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use image::Rgba;
use nalgebra::{point, Matrix4, Vector4};
use wgpu::{
    BufferDescriptor, BufferUsages, Device, ImageCopyBuffer, ImageDataLayout, Queue, Texture,
    TextureFormat, TextureView,
};

mod pipeline {
    use wgpu::{
        DepthStencilState, Device, PipelineLayout, RenderPipeline, ShaderModule, TextureFormat,
        VertexBufferLayout,
    };

    pub fn build_render_pipe_line<'a>(
        name: &str,
        device: &Device,
        render_pipeline_layout: &PipelineLayout,
        shader: &ShaderModule,
        buffer_layout_v: &[VertexBufferLayout<'a>],
        format: TextureFormat,
        topology: wgpu::PrimitiveTopology,
        depth_stencil_op: Option<DepthStencilState>,
    ) -> RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: buffer_layout_v,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                ..Default::default()
            },
            depth_stencil: depth_stencil_op,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }
}
mod body_render;
mod view_renderer;

pub mod camera;
pub mod err;
pub mod light_mapping;
pub mod structs;

pub const WGPU_OFFSET_M: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0,
);

pub enum ThreeLook {
    Body(Body),
    Light(Light),
}

impl ThreeLook {
    pub fn as_body(&self) -> Option<&Body> {
        if let ThreeLook::Body(buf) = self {
            return Some(buf);
        }

        None
    }

    pub fn as_body_mut(&mut self) -> Option<&mut Body> {
        if let ThreeLook::Body(buf) = self {
            return Some(buf);
        }

        None
    }

    pub fn as_light(&self) -> Option<&Light> {
        if let ThreeLook::Light(light) = self {
            return Some(light);
        }

        None
    }

    pub fn as_light_mut(&mut self) -> Option<&mut Light> {
        if let ThreeLook::Light(light) = self {
            return Some(light);
        }

        None
    }
}

pub struct Light {
    pub color: Vector4<f32>,
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
}

pub struct Body {
    pub model_m: Matrix4<f32>,
    pub buf: Arc<wgpu::Buffer>,
}

pub struct ThreeDrawer {
    light_mapping_builder: light_mapping::LightMappingBuilder,
    body_renderer: body_render::BodyRenderer,
    camera_state: camera::CameraState,
    proj_m: Matrix4<f32>,
    view_renderer: view_renderer::ViewRenderer,
}

impl ThreeDrawer {
    pub fn new(device: &Device, format: TextureFormat, proj_m: Matrix4<f32>) -> Self {
        let light_mapping_builder = light_mapping::LightMappingBuilder::new(device);
        let body_renderer = body_render::BodyRenderer::new(device, format);
        let view_renderer = view_renderer::ViewRenderer::new(device);

        Self {
            light_mapping_builder,
            body_renderer,
            camera_state: camera::CameraState::new(point![0.0, 0.0, 0.0], 0.0, 0.0),
            proj_m,
            view_renderer,
        }
    }

    pub fn render(
        &self,
        device: &Device,
        queue: &Queue,
        surface: &TextureView,
        look_v: Vec<&ThreeLook>,
        ratio: f32,
    ) -> err::Result<()> {
        let mut body_v = vec![];
        let mut light_v = vec![];

        for look in look_v {
            match look {
                ThreeLook::Body(buffer) => body_v.push(buffer),
                ThreeLook::Light(light) => light_v.push(light),
            }
        }

        // mapping of light_v
        let light_texture_v = light_v
            .iter()
            .map(|light| {
                (
                    *light,
                    self.light_mapping_builder.light_mapping(
                        device,
                        queue,
                        &(light.proj * light.view),
                        &body_v,
                    ),
                )
            })
            .collect::<Vec<(&Light, (Texture, Texture))>>();

        let view_m = self.camera_state.calc_matrix();

        // color and depth of view
        let view_texture =
            self.view_renderer
                .view_renderer(device, queue, &view_m, &self.proj_m, &body_v);

        self.body_renderer.body_render(
            device,
            queue,
            surface,
            view_texture,
            light_texture_v,
            &view_m,
            &self.proj_m,
            ratio,
        )
    }

    pub fn camera_state(&self) -> &camera::CameraState {
        &self.camera_state
    }

    pub fn camera_state_mut(&mut self) -> &mut camera::CameraState {
        &mut self.camera_state
    }
}

pub fn save_texture(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    path: &str,
    p_sz: usize,
    f: impl Fn(u32, u32, &[u8]) -> Rgba<u8>,
) {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let (tx, rx) = channel::<bool>();

    let buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: (texture.width() * texture.height() * p_sz as u32) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * p_sz as u32),
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
