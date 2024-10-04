use edge_lib::util::{
    data::{AsDataManager, MemDataManager, TempDataManager},
    engine::AsEdgeEngine,
};
use nalgebra::{vector, Matrix3, Vector2, Vector3};
use rapier2d::prelude::{
    Collider, ColliderBuilder, GenericJoint, IntegrationParameters, RigidBody, RigidBodyBuilder,
    RigidBodyHandle,
};
use view_manager::{AsViewManager, VNode, ViewProps};

use std::{collections::HashMap, sync::Arc};
use wgpu::{Instance, Surface};

use winit::{dpi::PhysicalSize, window::Window};

use crate::{err, util::shape};

use super::shape::Shape;

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

    pub fn gen_light_line_v(engine: &Engine) -> Vec<LineIn> {
        let mut line_v = Vec::new();
        let physics_manager = &engine.physics_manager;
        for (_, rigid_body) in physics_manager.physics_engine.rigid_body_set.iter() {
            let body_id = rigid_body.user_data as u64;
            for body_look in &physics_manager.body_mp[&body_id].look.light_look {
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

    pub fn gen_line_v(engine: &Engine) -> Vec<Line> {
        let scene = &engine.physics_manager;
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

pub mod builder;
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
    view_class: HashMap<String, Vec<String>>,
}

impl EngineBuilder {
    pub fn from_window(
        window: &'static Window,
        view_class: HashMap<String, Vec<String>>,
    ) -> err::Result<Self> {
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
            view_class,
        })
    }

    pub async fn build(self) -> err::Result<Engine> {
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

        Ok(Engine::new(
            Arc::new(MemDataManager::new(None)),
            self.view_class,
            res::AudioManager::new(),
            res::PhysicsManager::new(IntegrationParameters::default()),
            res::VisionManager::new(
                ray_drawer,
                watcher_drawer,
                surface_drawer,
                self.surface,
                device,
                queue,
                config,
            ),
        )
        .await)
    }
}

pub struct Engine {
    unique_id: u64,
    time_stamp: u128,
    watcher_binding_body_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, Vec<String>>,

    data_manager: TempDataManager,
    audio_manager: res::AudioManager,
    physics_manager: res::PhysicsManager,
    vision_manager: res::VisionManager,
}

impl Engine {
    pub async fn new(
        dm: Arc<dyn AsDataManager>,
        view_class: HashMap<String, Vec<String>>,
        audio_manager: res::AudioManager,
        physics_manager: res::PhysicsManager,
        vision_manager: res::VisionManager,
    ) -> Self {
        let mut this = Self {
            unique_id: 0,
            time_stamp: 0,
            watcher_binding_body_id: 0,
            vnode_mp: HashMap::new(),
            view_class,
            data_manager: TempDataManager::new(dm),
            audio_manager,
            physics_manager,
            vision_manager,
        };

        let root_id = this.new_vnode();
        this.apply_props(
            root_id,
            &ViewProps {
                class: format!("Main"),
                props: json::Null,
                child_v: vec![],
            },
        )
        .await
        .unwrap();

        this
    }

    pub async fn event_handler(
        &mut self,
        entry_name: &str,
        event: &json::JsonValue,
    ) -> err::Result<()> {
        match entry_name {
            "onresize" => {
                self.vision_manager.resize(PhysicalSize {
                    width: event["width"].as_i32().unwrap() as u32,
                    height: event["height"].as_i32().unwrap() as u32,
                });
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Step and render
    pub fn step(&mut self) -> err::Result<()> {
        step::step(self);

        // Update Watcher
        let watcher_body = self
            .physics_manager
            .body_mp
            .get(&self.watcher_binding_body_id)
            .ok_or(err::Error::Other(format!("no wather!")))?;
        let rigid_body = &self.physics_manager.physics_engine.rigid_body_set[watcher_body.rigid];
        let pos = rigid_body.translation();
        self.physics_manager.watcher.position[0] = pos.x;
        self.physics_manager.watcher.position[1] = pos.y;
        self.vision_manager
            .ray_drawer
            .update_watcher(&self.vision_manager.device, &self.physics_manager.watcher);

        // Update line
        let line_v = inner::gen_line_v(self);
        if !line_v.is_empty() {
            self.vision_manager
                .ray_drawer
                .update_line_v(&self.vision_manager.device, &line_v);

            // Draw ray tracing result to texture
            self.vision_manager
                .ray_drawer
                .draw_ray_to_point_texture(&self.vision_manager.device, &self.vision_manager.queue);
        }

        // Draw to surface
        let output = self
            .vision_manager
            .surface
            .get_current_texture()
            .map_err(err::map_append("\nat get_current_texture"))?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            // Draw point to surface
            self.vision_manager.surface_drawer.draw_point_to_surface(
                &self.vision_manager.device,
                &self.vision_manager.queue,
                &view,
                self.vision_manager.ray_drawer.get_result_buffer(),
                self.vision_manager.ray_drawer.get_size_buffer(),
            )?;
            // Draw watcher to surface
            self.vision_manager.light_drawer.draw_light_to_surface(
                &self.vision_manager.device,
                &self.vision_manager.queue,
                &view,
                self.vision_manager.ray_drawer.get_watcher_buffer(),
                self.vision_manager.ray_drawer.get_size_buffer(),
                &inner::gen_light_line_v(self),
            )?;
        }
        output.present();

        Ok(())
    }

    pub fn move_watcher(&mut self, offset: Vector2<f32>) {
        self.physics_manager.watcher.offset[0] += offset.x;
        self.physics_manager.watcher.offset[1] += offset.y;
        self.vision_manager
            .ray_drawer
            .update_watcher(&self.vision_manager.device, &self.physics_manager.watcher);
    }

    //// Add a body into this scene.
    pub fn add_body(&mut self, mut body: BodyBuilder) -> u64 {
        let body_id = self.unique_id;
        self.unique_id += 1;
        let scene = &mut self.physics_manager;
        body.rigid.user_data = body_id as u128;
        let body_handle = scene.physics_engine.rigid_body_set.insert(body.rigid);
        scene.body_mp.insert(
            body_id,
            Body {
                class: body.class.clone(),
                name: body.name.clone(),
                look: body.look,
                rigid: body_handle,
                life_step_op: body.life_step_op,
            },
        );
        match scene.body_index_mp.get_mut(&body.class) {
            Some(mp) => {
                mp.insert(body.name, body_id);
            }
            None => {
                let mut mp = HashMap::new();
                mp.insert(body.name, body_id);
                scene.body_index_mp.insert(body.class.clone(), mp);
            }
        }
        for collider in body.collider.collider_v {
            scene.physics_engine.collider_set.insert_with_parent(
                collider,
                body_handle,
                &mut scene.physics_engine.rigid_body_set,
            );
        }
        body_id
    }

    pub fn remove_body(&mut self, id: &u64) {
        self.physics_manager.remove_body(id)
    }
}

impl AsEdgeEngine for Engine {
    fn get_dm(&self) -> &TempDataManager {
        &self.data_manager
    }

    fn get_dm_mut(&mut self) -> &mut TempDataManager {
        &mut self.data_manager
    }

    fn reset(&mut self) {
        self.data_manager.temp = Arc::new(MemDataManager::new(None));
    }
}

impl AsViewManager for Engine {
    fn get_class(&self, class: &str) -> Option<&Vec<String>> {
        self.view_class.get(class)
    }

    fn get_vnode(&self, id: &u64) -> Option<&view_manager::VNode> {
        self.vnode_mp.get(id)
    }

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut view_manager::VNode> {
        self.vnode_mp.get_mut(id)
    }

    fn new_vnode(&mut self) -> u64 {
        let new_id = self.unique_id;
        self.unique_id += 1;
        self.vnode_mp.insert(
            new_id,
            VNode::new(ViewProps {
                class: format!(""),
                props: json::Null,
                child_v: vec![],
            }),
        );
        new_id
    }

    fn rm_vnode(&mut self, id: u64) -> Option<view_manager::VNode> {
        self.vnode_mp.remove(&id)
    }

    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps) {
        let vnode = self.get_vnode(&id).unwrap();

        let mut need_update_watcher = false;
        if let Some(is_watcher) = props.props["$:watcher"][0].as_str() {
            if is_watcher == "true" {
                need_update_watcher = true;
            }
        }

        let body_id_op = if vnode.view_props.class != props.class {
            log::debug!(
                "change {} to {} at vnode:{id}",
                vnode.view_props.class,
                props.class
            );
            // delete body
            match vnode.view_props.class.as_str() {
                "ball" => {
                    if let Some(body_id) = self
                        .physics_manager
                        .get_body_id_by_class_name("ball", &format!("{id}"))
                    {
                        self.remove_body(&body_id);
                    }
                }
                _ => (),
            }

            // insert body
            match props.class.as_str() {
                "ball" => {
                    let body_id = self.add_body(BodyBuilder::new(
                        "ball".to_string(),
                        format!("{id}"),
                        BodyLook {
                            ray_look: vec![],
                            light_look: vec![LightLook {
                                shape: Shape::circle(),
                                shape_matrix: Matrix3::new_scaling(0.05),
                                color: Vector3::new(1.0, 0.0, 1.0),
                                is_visible: true,
                            }],
                        },
                        BodyCollider {
                            collider_v: vec![ColliderBuilder::ball(0.05).mass(0.001).build()],
                        },
                        RigidBodyBuilder::dynamic()
                            .ccd_enabled(true)
                            .translation(vector![0.0, 0.2])
                            .build(),
                        None,
                    ));
                    Some(body_id)
                }
                _ => None,
            }
        } else {
            None
        };

        if let Some(body_id) = body_id_op {
            log::debug!("add body {body_id} to {id} {}", props.class);
            if need_update_watcher {
                log::debug!("set {body_id} as watcher");
                self.watcher_binding_body_id = body_id;
            }
        }
    }
}
