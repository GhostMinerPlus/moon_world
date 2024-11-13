//! imported => [Engine] = avaliable to render

use error_stack::ResultExt;
use moon_class::{util::rs_2_str, AsClassManager, Fu};
use nalgebra::{point, vector, Matrix4};
use rapier3d::prelude::{IntegrationParameters, RigidBodyHandle};
use view_manager::{AsElementProvider, AsViewManager, VNode, ViewProps};

use std::{collections::HashMap, future::Future, pin::Pin};
use wgpu::{Instance, Surface};

use winit::{dpi::PhysicalSize, window::Window};

mod physics;
mod res;
mod inner {
    use std::collections::HashMap;

    use error_stack::ResultExt;
    use view_manager::VNode;

    use crate::err;

    use super::{res::RenderPass, AtomElement};

    /// Let vnode be rendered.
    pub fn render_vnode(
        vnode_mp: &HashMap<u64, VNode>,
        element_mp: &HashMap<u64, AtomElement>,
        rp: &mut RenderPass,
        vnode_id: u64,
    ) -> err::Result<()> {
        let vnode = vnode_mp.get(&vnode_id).unwrap();
        if vnode.inner_node.data != 0 {
            // Let virtual container be rendered.
            render_vnode(vnode_mp, element_mp, rp, vnode.inner_node.data)
        } else {
            // Let meta container or meta tag be rendered.
            match vnode.view_props.class.as_str() {
                "div" => {
                    for child_node in vnode.embeded_child_v.clone() {
                        render_vnode(vnode_mp, element_mp, rp, child_node)?;
                    }
                }
                _ => {
                    let ele = element_mp
                        .get(&vnode_id)
                        .ok_or(err::Error::NotFound)
                        .attach_printable("element with specified vnode_id not found!")?;
                    match ele {
                        super::AtomElement::Audio(_) => (),
                        super::AtomElement::Vision(id) => {
                            rp.push_element(*id);
                        }
                        _ => (),
                    }
                }
            }

            Ok(())
        }
    }
}

pub mod dep;
pub mod err;
pub mod util;

/// built => the result = a new [Engine]
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
            .change_context(err::Error::Other)?;

        Ok(Self {
            instance,
            surface,
            size,
        })
    }

    /// called => the [EngineBuilder] = built
    pub async fn build(self, dm: Box<dyn AsClassManager>) -> err::Result<Engine> {
        let adapter = self
            .instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&self.surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(err::Error::NotFound)?;

        let (device, queue) = {
            adapter
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
                .change_context(err::Error::Other)?
        };

        log::debug!("found device: {:?}", device);

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
                .ok_or(err::Error::NotFound)?;

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

        Ok(Engine::new(
            dm,
            res::PhysicsManager::new(IntegrationParameters::default()),
            res::VisionManager::new(self.surface, device, queue, config),
        ))
    }
}

pub enum AtomElement {
    Audio(()),
    Physics(RigidBodyHandle),
    Vision(u64),
    Input(u64),
}

/// stepped => time = next time
///
/// rendered => frame = next frame
pub struct Engine {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    watcher_binding_body_id: u64,
    element_mp: HashMap<u64, AtomElement>,

    data_manager: Box<dyn AsClassManager>,
    physics_manager: res::PhysicsManager,
    vision_manager: res::VisionManager,
    input_provider: res::InputProvider,
}

impl Engine {
    /// called => the result = a new [Engine]
    pub fn new(
        dm: Box<dyn AsClassManager>,
        physics_manager: res::PhysicsManager,
        vision_manager: res::VisionManager,
    ) -> Self {
        Self {
            unique_id: 0,
            vnode_mp: HashMap::new(),
            watcher_binding_body_id: 0,
            element_mp: HashMap::new(),
            data_manager: dm,
            physics_manager,
            vision_manager,
            input_provider: res::InputProvider::new(),
        }
    }

    pub async fn init(&mut self, entry: ViewProps) {
        let root_id = self.new_vnode(0);
        self.apply_props(root_id, &entry, 0, true).await.unwrap();
    }

    /// called => the event = handled[]
    pub async fn event_handler(
        &mut self,
        entry_name: &str,
        data: &json::JsonValue,
    ) -> err::Result<()> {
        for id in self
            .element_mp
            .iter()
            .filter(|(_, ele)| {
                if let AtomElement::Input(_) = ele {
                    return true;
                }
                false
            })
            .map(|(id, _)| *id)
            .collect::<Vec<u64>>()
        {
            let _ = self
                .event_entry(id, entry_name, data)
                .await
                .change_context(err::Error::Other)?;
        }

        Ok(())
    }

    /// called => the engine = stepped
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
            let _ = self.event_entry(id, "$onstep", &json::Null).await;
        }

        Ok(())
    }

    /// called => the engine = rendered
    pub fn render(&mut self) -> err::Result<()> {
        let mut rp = self.vision_manager.render_pass()?;

        inner::render_vnode(&self.vnode_mp, &self.element_mp, &mut rp, 0)?;

        rp.render()
    }
}

impl AsClassManager for Engine {
    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        item_v: Vec<String>,
    ) -> std::pin::Pin<Box<dyn Fu<Output = moon_class::err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            if class.starts_with('@') {
                if class == "@new_size" && source == "@window" {
                    let data = json::parse(&rs_2_str(&item_v)).unwrap();

                    self.vision_manager.resize(PhysicalSize {
                        width: data["$width"][0].as_str().unwrap().parse().unwrap(),
                        height: data["$height"][0].as_str().unwrap().parse().unwrap(),
                    });

                    return Ok(());
                } else if class == "@new_step" && source == "@camera" {
                    let data = json::parse(&rs_2_str(&item_v)).unwrap();

                    log::debug!("@new_step: data = {data}");

                    *self.vision_manager.view_m_mut() = Matrix4::new_translation(&vector![
                        -data["$x"][0].as_str().unwrap().parse::<f32>().unwrap(),
                        -data["$y"][0].as_str().unwrap().parse::<f32>().unwrap(),
                        -data["$z"][0].as_str().unwrap().parse::<f32>().unwrap(),
                    ]) * self.vision_manager.view_m();

                    return Ok(());
                }
            }

            self.data_manager.append(class, source, item_v).await
        })
    }

    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> std::pin::Pin<Box<dyn Fu<Output = moon_class::err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.data_manager.clear(class, source)
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> std::pin::Pin<Box<dyn Fu<Output = moon_class::err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        Box::pin(async move {
            match class {
                "@moon_world_pos" => {
                    let vnode_id = source.parse::<u64>().unwrap();

                    let ele = self.element_mp.get(&vnode_id).unwrap();
                    if let AtomElement::Physics(h) = ele {
                        let pos = self
                            .physics_manager
                            .physics_engine
                            .rigid_body_set
                            .get(*h)
                            .unwrap()
                            .translation();

                        Ok(vec![
                            pos.x.to_string(),
                            pos.y.to_string(),
                            pos.z.to_string(),
                        ])
                    } else {
                        Err(moon_class::err::Error::NotFound).attach_printable_lazy(|| {
                            format!("not such AtomElement with id {vnode_id}")
                        })
                    }
                }
                "@camera_pos" => {
                    let pos = self
                        .vision_manager
                        .view_m()
                        .transform_point(&point![0.0, 0.0, 0.0]);

                    Ok(vec![
                        pos.x.to_string(),
                        pos.y.to_string(),
                        pos.z.to_string(),
                    ])
                }
                _ => self.data_manager.get(class, source).await,
            }
        })
    }

    fn get_source<'a, 'a1, 'a2, 'f>(
        &'a self,
        target: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn Fu<Output = moon_class::err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.data_manager.get_source(target, class)
    }
}

impl AsElementProvider for Engine {
    type H = u64;

    /// Element generator, let the variable be id of the new element which consists of physics, vision and audio.
    fn create_element(&mut self, vnode_id: u64, class: &str, props: &json::JsonValue) -> u64 {
        let (prefix, suffix) = match class.find(':') {
            Some(pos) => (&class[0..pos], &class[pos + 1..]),
            None => ("", class),
        };

        let atom_element = match prefix {
            "Physics" => {
                AtomElement::Physics(self.physics_manager.create_element(vnode_id, suffix, props))
            }
            "Vision" => {
                AtomElement::Vision(self.vision_manager.create_element(vnode_id, suffix, props))
            }
            "Input" => {
                AtomElement::Input(self.input_provider.create_element(vnode_id, suffix, props))
            }
            _ => {
                return vnode_id;
            }
        };

        self.element_mp.insert(vnode_id, atom_element);

        vnode_id
    }

    /// Let the element specified by the id be deleted.
    fn delete_element(&mut self, id: u64) {
        if let Some(atom_ele) = self.element_mp.remove(&id) {
            match atom_ele {
                AtomElement::Audio(_) => todo!(),
                AtomElement::Physics(rigid_body_handle) => {
                    self.physics_manager.delete_element(rigid_body_handle)
                }
                AtomElement::Vision(id) => self.vision_manager.delete_element(id),
                AtomElement::Input(id) => self.input_provider.delete_element(id),
            }
        }
    }

    /// Let the element specified by the id be updated by this props.
    fn update_element(&mut self, id: u64, class: &str, props: &json::JsonValue) {
        let (_, suffix) = match class.find(':') {
            Some(pos) => (&class[0..pos], &class[pos + 1..]),
            None => ("", class),
        };

        if let Some(atom_ele) = self.element_mp.get_mut(&id) {
            match atom_ele {
                AtomElement::Audio(_) => todo!(),
                AtomElement::Physics(rigid_body_handle) => {
                    self.physics_manager
                        .update_element(*rigid_body_handle, suffix, props);
                    if let Some(watcher) = props["$watcher"][0].as_str() {
                        if watcher == "true" {
                            self.watcher_binding_body_id = id;
                        }
                    }
                }
                AtomElement::Vision(id) => {
                    self.vision_manager.update_element(*id, suffix, props);
                }
                AtomElement::Input(id) => {
                    self.input_provider.update_element(*id, suffix, props);
                }
            }
        }
    }
}

impl AsViewManager for Engine {
    fn get_class_view<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 str,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let rs = self.get("view", class).await.unwrap();
            if rs.is_empty() {
                None
            } else {
                Some(rs_2_str(&rs))
            }
        })
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
}
