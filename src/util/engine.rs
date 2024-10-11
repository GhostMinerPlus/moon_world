//! Help the crate be a video provider or a event handler.
//! - video provider > frame provider + step

use edge_lib::util::data::{AsDataManager, AsStack, MemDataManager, TempDataManager};
use nalgebra::{Matrix3, Vector2, Vector3};
use rapier2d::prelude::{
    Collider, GenericJoint, IntegrationParameters, RigidBody, RigidBodyHandle,
};
use structs::Watcher;
use view_manager::util::{AsViewManager, VNode, ViewProps};

use std::{collections::HashMap, io};
use wgpu::{Instance, Surface};

use winit::{dpi::PhysicalSize, window::Window};

use crate::{err, util::shape};

mod drawer;
mod physics;
mod res;
mod structs;

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
    pub look: BodyLook,
    pub life_step_op: Option<u64>,
    pub matrix: Matrix3<f32>,
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
            Box::new(MemDataManager::new(None)),
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

pub enum AtomElement {
    Data(String),
    Audio(()),
    Physics(RigidBodyHandle),
    Vision(u64),
}

/// A frame provider and a event handler.
pub struct Engine {
    unique_id: u64,
    time_stamp: u128,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, Vec<String>>,
    watcher_binding_body_id: u64,
    element_mp: HashMap<u64, AtomElement>,
    element_index_mp: HashMap<String, HashMap<String, u64>>,
    watcher: Watcher,

    data_manager: TempDataManager,
    audio_manager: res::AudioManager,
    physics_manager: res::PhysicsManager,
    vision_manager: res::VisionManager,
}

impl Engine {
    /// [Engine] constructor.
    pub async fn new(
        dm: Box<dyn AsDataManager>,
        view_class: HashMap<String, Vec<String>>,
        audio_manager: res::AudioManager,
        physics_manager: res::PhysicsManager,
        vision_manager: res::VisionManager,
    ) -> Self {
        let mut this = Self {
            unique_id: 0,
            time_stamp: 0,
            vnode_mp: HashMap::new(),
            view_class,
            watcher_binding_body_id: 0,
            element_mp: HashMap::new(),
            element_index_mp: HashMap::new(),
            watcher: Watcher {
                position: [0.0, 0.0],
                offset: [0.0, 0.0],
            },
            data_manager: TempDataManager::new(dm),
            audio_manager,
            physics_manager,
            vision_manager,
        };

        let root_id = this.new_vnode(0);
        this.apply_props(
            root_id,
            &ViewProps {
                class: format!("Main"),
                props: json::Null,
            },
            0,
            true,
        )
        .await
        .unwrap();

        this
    }

    /// Event handler, let event be handled.
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

    /// Let the engine be stepped.
    pub async fn step(&mut self) -> err::Result<()> {
        self.physics_manager.step();

        for id in self
            .element_mp
            .iter()
            .filter(|(_, ele)| {
                if let AtomElement::Physics(_) = ele {
                    return true;
                }
                false
            })
            .map(|(id, _)| *id)
            .collect::<Vec<u64>>()
        {
            let _ = self.event_entry(id, "$:onstep", json::Null).await;
        }
        Ok(())
    }

    /// Let the engine be rendered.
    pub fn render(&mut self) -> err::Result<()> {
        if let Some(ele) = self.element_mp.get(&self.watcher_binding_body_id) {
            if let AtomElement::Physics(h) = ele {
                if let Some(body) = self.physics_manager.physics_engine.rigid_body_set.get(*h) {
                    let pos = body.translation();

                    self.watcher.position = [pos.x, pos.y];
                }
            }
        }

        // Let the surface be drew.
        self.vision_manager.render(&self.watcher)
    }

    pub fn move_watcher(&mut self, offset: Vector2<f32>) {
        self.physics_manager.watcher.offset[0] += offset.x;
        self.physics_manager.watcher.offset[1] += offset.y;
        self.vision_manager
            .ray_drawer
            .update_watcher(&self.vision_manager.device, &self.physics_manager.watcher);
    }

    /// Element generator, let the variable be id of the new element which consists of physics, vision and audio.
    pub fn create_element(&mut self, id: u64, class: &str) {
        let atom_element = if class.starts_with("Physics:") {
            match class {
                "Physics:ball" => {
                    AtomElement::Physics(self.physics_manager.create_element("ball").unwrap())
                }
                _ => {
                    return;
                }
            }
        } else if class.starts_with("Vision:") {
            match class {
                "Vision:ball" => {
                    AtomElement::Vision(self.vision_manager.create_element("ball").unwrap())
                }
                _ => {
                    return;
                }
            }
        } else {
            return;
        };
        self.element_mp.insert(id, atom_element);
    }

    /// Let the element specified by the id be deleted.
    pub fn delete_element(&mut self, id: u64) {
        if let Some(atom_ele) = self.element_mp.remove(&id) {
            match atom_ele {
                AtomElement::Data(_) => todo!(),
                AtomElement::Audio(_) => todo!(),
                AtomElement::Physics(rigid_body_handle) => {
                    self.physics_manager.delete_element(rigid_body_handle)
                }
                AtomElement::Vision(id) => self.vision_manager.delete_element(id),
            }
        }
    }

    /// Let the element specified by the id be updated by this props.
    pub fn update_element(&mut self, id: u64, props: &ViewProps) {
        if let Some(atom_ele) = self.element_mp.get_mut(&id) {
            match atom_ele {
                AtomElement::Data(_) => todo!(),
                AtomElement::Audio(_) => todo!(),
                AtomElement::Physics(rigid_body_handle) => {
                    self.physics_manager
                        .update_element(*rigid_body_handle, props);
                    if let Some(watcher) = props.props["$:watcher"][0].as_str() {
                        if watcher == "true" {
                            self.watcher_binding_body_id = id;
                        }
                    }
                }
                AtomElement::Vision(id) => {
                    self.vision_manager.update_element(*id, props);
                }
            }
        }
    }
}

impl AsDataManager for Engine {
    fn call<'a, 'a1, 'a2, 'a3, 'a4, 'f>(
        &'a mut self,
        output: &'a1 edge_lib::util::Path,
        func: &'a2 str,
        input: &'a3 edge_lib::util::Path,
        input1: &'a4 edge_lib::util::Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
        'a3: 'f,
        'a4: 'f,
    {
        Box::pin(async move {
            match func {
                "$world2_get_pos" => {
                    let vnode_id = self
                        .get(&input)
                        .await?
                        .first()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();
                    let ele = self.element_mp.get(&vnode_id).unwrap();
                    if let AtomElement::Physics(h) = ele {
                        let pos = self
                            .physics_manager
                            .physics_engine
                            .rigid_body_set
                            .get(*h)
                            .unwrap()
                            .translation();

                        self.set(output, vec![pos.x.to_string(), pos.y.to_string()])
                            .await
                    } else {
                        Err(io::Error::other(format!("no an AtomElement::Physics")))
                    }
                }
                _ => self.data_manager.call(output, func, input, input1).await,
            }
        })
    }

    fn get_auth(&self) -> &edge_lib::util::data::Auth {
        self.data_manager.get_auth()
    }

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        path: &'a1 edge_lib::util::Path,
        item_v: Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        self.data_manager.append(path, item_v)
    }

    fn set<'a, 'a1, 'f>(
        &'a mut self,
        path: &'a1 edge_lib::util::Path,
        item_v: Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        self.data_manager.set(path, item_v)
    }

    fn get<'a, 'a1, 'f>(
        &'a self,
        path: &'a1 edge_lib::util::Path,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<String>>> + Send + 'f>,
    >
    where
        'a: 'f,
        'a1: 'f,
    {
        self.data_manager.get(path)
    }

    fn get_code_v<'a, 'a1, 'a2, 'f>(
        &'a self,
        root: &'a1 str,
        space: &'a2 str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<String>>> + Send + 'f>,
    >
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.data_manager.get_code_v(root, space)
    }
}

impl AsStack for Engine {
    fn push<'a, 'f>(
        &'a mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>> {
        self.data_manager.push()
    }

    fn pop<'a, 'f>(
        &'a mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>> {
        self.data_manager.pop()
    }
}

impl AsViewManager for Engine {
    fn get_class(&self, class: &str) -> Option<&Vec<String>> {
        self.view_class.get(class)
    }

    fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.vnode_mp.get(id)
    }

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode> {
        self.vnode_mp.get_mut(id)
    }

    fn new_vnode(&mut self, context: u64) -> u64 {
        let new_id = self.unique_id;
        self.unique_id += 1;
        self.vnode_mp.insert(new_id, VNode::new(context));
        new_id
    }

    fn rm_vnode(&mut self, id: u64) -> Option<VNode> {
        self.vnode_mp.remove(&id)
    }

    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps) {
        // Let the element be usable.
        if self.get_vnode(&id).unwrap().view_props.class != props.class {
            self.delete_element(id);

            self.create_element(id, &props.class);
        }

        // Let the element be updated.
        self.update_element(id, props);
    }
}