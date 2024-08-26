use handle::SceneHandle;
use nalgebra::{Matrix3, Vector2, Vector3};
use rapier2d::prelude::{Collider, GenericJoint, IntegrationParameters, RigidBody, RigidBodyHandle};
use rodio::{cpal::FromSample, OutputStream, OutputStreamHandle, Sample, Sink, Source};

use std::collections::HashMap;
use wgpu::{Instance, Surface};

use winit::{dpi::PhysicalSize, window::Window};

use crate::{err, shape};

mod drawer;
mod physics;
mod res;
mod step;
mod structs;
mod inner {
    use nalgebra::{Matrix3, Point2, Vector2};

    use super::{
        structs::{Line, LineIn},
        Engine,
    };

    pub fn gen_light_line_v<D, E>(engine: &Engine<D, E>) -> Vec<LineIn> {
        let mut line_v = Vec::new();
        let scene = &engine.scene_mp[&engine.cur_scene_id];
        for (_, rigid_body) in scene.physics_engine.rigid_body_set.iter() {
            let body_id = rigid_body.user_data as u64;
            for body_look in &scene.body_mp[&body_id].look.light_look {
                if !body_look.is_visible {
                    continue;
                }
                let body_matrix = {
                    let position = rigid_body.translation();
                    let angle = rigid_body.rotation().angle();
                    let body_matrix =
                        Matrix3::new_translation(&Vector2::new(position.x, position.y))
                            * Matrix3::new_rotation(angle);
                    body_matrix
                };
                let matrix = body_matrix * body_look.shape_matrix;
                let point_v = body_look
                    .shape
                    .point_v
                    .iter()
                    .map(|point| matrix.transform_point(point))
                    .collect::<Vec<Point2<f32>>>();
                if point_v.is_empty() {
                    return line_v;
                }
                for i in 0..point_v.len() - 1 {
                    let sp = point_v[i];
                    let ep = point_v[i + 1];
                    line_v.push(LineIn {
                        position: [sp.x, sp.y],
                        color: body_look.color.into(),
                    });
                    line_v.push(LineIn {
                        position: [ep.x, ep.y],
                        color: body_look.color.into(),
                    });
                }
            }
        }

        line_v
    }

    pub fn gen_line_v<D, E>(engine: &Engine<D, E>) -> Vec<Line> {
        let scene = &engine.scene_mp[&engine.cur_scene_id];
        let mut line_v = Vec::new();
        for (_, rigid_body) in scene.physics_engine.rigid_body_set.iter() {
            let body_id = rigid_body.user_data as u64;
            for body_look in &scene.body_mp[&body_id].look.ray_look {
                if !body_look.is_visible {
                    continue;
                }
                let body_matrix = {
                    let position = rigid_body.translation();
                    let angle = rigid_body.rotation().angle();
                    let body_matrix =
                        Matrix3::new_translation(&Vector2::new(position.x, position.y))
                            * Matrix3::new_rotation(angle);
                    body_matrix
                };
                let matrix = body_matrix * body_look.shape_matrix;
                let point_v = body_look
                    .shape
                    .point_v
                    .iter()
                    .map(|point| matrix.transform_point(point))
                    .collect::<Vec<Point2<f32>>>();
                if point_v.is_empty() {
                    continue;
                }
                for i in 0..point_v.len() - 1 {
                    let sp = point_v[i];
                    let ep = point_v[i + 1];
                    line_v.push(Line {
                        sp: sp.into(),
                        ep: ep.into(),
                        light: body_look.light,
                        color: body_look.color.into(),
                        roughness: body_look.roughness,
                        seed: body_look.seed + i as f32,
                        ..Default::default()
                    });
                }
            }
        }
        line_v
    }
}

pub mod handle;

#[derive(Clone)]
pub struct BodyLook {
    pub ray_look: Vec<RayLook>,
    pub light_look: Vec<LightLook>,
}

#[derive(Clone)]
pub struct LightLook {
    pub shape: shape::Shape,
    pub shape_matrix: Matrix3<f32>,
    pub color: Vector3<f32>,
    pub is_visible: bool,
}

#[derive(Clone)]
pub struct RayLook {
    pub shape: shape::Shape,
    pub shape_matrix: Matrix3<f32>,
    pub color: Vector3<f32>,
    pub light: f32,
    pub roughness: f32,
    pub seed: f32,
    pub is_visible: bool,
}

#[derive(Clone)]
pub struct BodyCollider {
    pub collider_v: Vec<Collider>,
}

#[derive(Clone)]
pub struct BodyBuilder {
    class: String,
    name: String,
    look: BodyLook,
    collider: BodyCollider,
    rigid: RigidBody,
    life_step_op: Option<u64>,
}

impl BodyBuilder {
    pub fn new(
        class: String,
        name: String,
        look: BodyLook,
        collider: BodyCollider,
        rigid: RigidBody,
        life_step_op: Option<u64>,
    ) -> Self {
        Self {
            class,
            name,
            look,
            collider,
            rigid,
            life_step_op,
        }
    }
}

pub struct Body {
    pub class: String,
    pub name: String,
    pub look: BodyLook,
    pub rigid: RigidBodyHandle,
    pub life_step_op: Option<u64>,
}

pub struct Joint {
    pub body1: u64,
    pub body2: u64,
    pub joint: GenericJoint,
}

pub struct EngineBuilder {
    instance: Instance,
    surface: Surface<'static>,
    size: PhysicalSize<u32>,
}

impl EngineBuilder {
    pub fn from_window(window: &'static Window) -> err::Result<Self> {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::default(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .map_err(err::map_append("\nat create_surface"))?;
        Ok(Self {
            instance,
            surface,
            size,
        })
    }

    pub async fn build<D: Default, E>(self) -> err::Result<Engine<D, E>> {
        self.build_with(D::default()).await
    }

    pub async fn build_with<D, E>(self, user_data: D) -> err::Result<Engine<D, E>> {
        let adapter = self
            .instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&self.surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(err::Error::Other("no adapter".to_string()))?;

        let (device, queue) = {
            adapter
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
                .map_err(err::map_append("\nat request_device"))?
        };
        log::info!("found device: {:?}", device);

        let config = {
            let surface_caps = self.surface.get_capabilities(&adapter);
            // Shader code in this tutorial assumes an sRGB surface texture. Using a different
            // one will result all the colors coming out darker. If you want to support non
            // sRGB surfaces, you'll need to account for that when drawing to the frame.
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .filter(|f| f.is_srgb())
                .next()
                .ok_or(err::Error::Other("no surface_caps.formats".to_string()))?;
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: self.size.width,
                height: self.size.height,
                present_mode: surface_caps.present_modes[0],
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            self.surface.configure(&device, &config);
            log::info!("prepared surface: {:?}", config);
            config
        };

        let surface_drawer = drawer::SurfaceDrawer::new(&device, &config);

        let watcher_drawer = drawer::WathcerDrawer::new(&device, &config);

        let ray_drawer = drawer::RayDrawer::new(&device, self.size);

        let (output_stream, output_stream_handle) = OutputStream::try_default().unwrap();

        Ok(Engine {
            ray_drawer,
            light_drawer: watcher_drawer,
            surface_drawer,
            unique_id: 0,
            device,
            queue,
            config,
            surface: self.surface,
            scene_mp: HashMap::new(),
            cur_scene_id: 0,
            watcher_binding_body_id: 0,
            time_stamp: 0,
            _output_stream: output_stream,
            output_stream_handle,
            user_data,
        })
    }
}

pub struct Engine<D, E> {
    unique_id: u64,
    scene_mp: HashMap<u64, res::Scene<D, E>>,

    cur_scene_id: u64,
    /// The id of body which bound by the watcher
    watcher_binding_body_id: u64,

    ray_drawer: drawer::RayDrawer,
    light_drawer: drawer::WathcerDrawer,
    surface_drawer: drawer::SurfaceDrawer,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    time_stamp: u128,

    _output_stream: OutputStream,
    output_stream_handle: OutputStreamHandle,

    pub user_data: D,
}

impl<D, E> Engine<D, E> {
    pub fn new_scene(&mut self, integration_parameters: IntegrationParameters) -> handle::SceneHandle<D, E> {
        let scene_id = self.unique_id;
        self.scene_mp.insert(scene_id, res::Scene::new(integration_parameters));
        self.unique_id += 1;
        handle::SceneHandle {
            engine: self,
            scene_id,
        }
    }

    pub fn set_scene(&mut self, scene_id: u64) {
        self.cur_scene_id = scene_id;
        let scene = self.scene_mp.get_mut(&self.cur_scene_id).unwrap();
        self.ray_drawer.update_watcher(&self.device, &scene.watcher);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.ray_drawer.resize(&self.device, &self.queue, new_size);
        }
    }

    pub fn move_watcher(&mut self, offset: Vector2<f32>) {
        let scene = self.scene_mp.get_mut(&self.cur_scene_id).unwrap();
        scene.watcher.offset[0] += offset.x;
        scene.watcher.offset[1] += offset.y;
        self.ray_drawer.update_watcher(&self.device, &scene.watcher);
    }

    /// Render
    pub fn render(&mut self) -> err::Result<()> {
        step::step(self);

        // Update Watcher
        let scene = self.scene_mp.get_mut(&self.cur_scene_id).unwrap();
        let rigid_body = &scene.physics_engine.rigid_body_set
            [scene.body_mp[&self.watcher_binding_body_id].rigid];
        let pos = rigid_body.translation();
        scene.watcher.position[0] = pos.x;
        scene.watcher.position[1] = pos.y;
        self.ray_drawer.update_watcher(&self.device, &scene.watcher);
        // Update line
        let line_v = inner::gen_line_v(self);
        self.ray_drawer.update_line_v(&self.device, &line_v);

        // Draw ray tracing result to texture
        self.ray_drawer
            .draw_ray_to_point_texture(&self.device, &self.queue);

        // Draw to surface
        let output = self
            .surface
            .get_current_texture()
            .map_err(err::map_append("\nat get_current_texture"))?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            // Draw point to surface
            self.surface_drawer.draw_point_to_surface(
                &self.device,
                &self.queue,
                &view,
                self.ray_drawer.get_result_buffer(),
                self.ray_drawer.get_size_buffer(),
            )?;
            // Draw watcher to surface
            self.light_drawer.draw_light_to_surface(
                &self.device,
                &self.queue,
                &view,
                self.ray_drawer.get_watcher_buffer(),
                self.ray_drawer.get_size_buffer(),
                &inner::gen_light_line_v(self),
            )?;
        }
        output.present();

        Ok(())
    }

    pub fn on_user_event(&mut self, event: E) {
        let scene = self.scene_mp.get(&self.cur_scene_id).unwrap();
        if scene.on_event.is_none() {
            return;
        }
        let scene_id = self.cur_scene_id;
        let listener = scene.on_event.as_ref().unwrap().clone();
        (*listener)(
            SceneHandle {
                engine: self,
                scene_id,
            },
            event,
        );
    }

    pub fn get_watcher_rigid_body_mut(&mut self) -> Option<&mut RigidBody> {
        let scene = self.scene_mp.get_mut(&self.cur_scene_id).unwrap();
        scene
            .physics_engine
            .rigid_body_set
            .get_mut(scene.body_mp[&self.watcher_binding_body_id].rigid)
    }

    pub fn get_watcher_binding_body_id(&self) -> u64 {
        self.watcher_binding_body_id
    }

    pub fn get_current_scene_handle_mut(&mut self) -> SceneHandle<D, E> {
        let scene_id = self.cur_scene_id;
        SceneHandle {
            engine: self,
            scene_id,
        }
    }

    /// Mix a sound into this engine.
    pub fn mix_sound<S>(&self, source: S) -> Sink
    where
        S: Source + Send + 'static,
        f32: FromSample<S::Item>,
        S::Item: Sample + Send,
    {
        let sink = Sink::try_new(&self.output_stream_handle).unwrap();
        sink.append(source);
        sink
    }
}

#[cfg(test)]
mod test_rodio {
    #[test]
    fn test() {
        use rodio::source::{SineWave, Source};
        use rodio::{OutputStream, Sink};
        use std::time::Duration;

        // _stream must live as long as the sink
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Add a dummy source of the sake of the example.
        let source = SineWave::new(440.0)
            .take_duration(Duration::from_secs_f32(0.25))
            .amplify(0.20);
        sink.append(source);

        // The sound plays in a separate thread. This call will block the current thread until the sink
        // has finished playing all its queued sounds.
        sink.sleep_until_end();
    }
}
