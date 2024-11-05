use std::{
    collections::HashMap,
    f32::consts::PI,
    sync::{mpsc::channel, Arc},
};

use drawer::{
    structs::{self, Watcher},
    Light, ThreeLook,
};
use error_stack::ResultExt;
use nalgebra::{vector, Matrix3, Matrix4, Vector3};
use rapier2d::prelude::{
    ColliderBuilder, IntegrationParameters, RigidBodyBuilder, RigidBodyHandle,
};
use view_manager::{AsElementProvider, ViewProps};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, SurfaceTexture,
};

use crate::{err, util::shape::Shape};

use super::{
    physics,
    util::{Body, BodyLook, RayLook},
};

mod inner {
    use std::sync::mpsc::Sender;

    use rapier2d::prelude::{
        Collider, ContactForceEvent, EventHandler, RigidBody, RigidBodyHandle,
    };

    use super::PhysicsManager;

    pub struct InnerEventHandler {
        collision_sender: Sender<rapier2d::prelude::CollisionEvent>,
        force_sender: Sender<rapier2d::prelude::ContactForceEvent>,
    }

    impl InnerEventHandler {
        pub fn new(
            collision_sender: Sender<rapier2d::prelude::CollisionEvent>,
            force_sender: Sender<rapier2d::prelude::ContactForceEvent>,
        ) -> Self {
            Self {
                collision_sender,
                force_sender,
            }
        }
    }

    impl EventHandler for InnerEventHandler {
        fn handle_collision_event(
            &self,
            _bodies: &rapier2d::prelude::RigidBodySet,
            _colliders: &rapier2d::prelude::ColliderSet,
            event: rapier2d::prelude::CollisionEvent,
            _contact_pair: Option<&rapier2d::prelude::ContactPair>,
        ) {
            let _ = self.collision_sender.send(event);
            log::debug!("sent collision_event");
        }

        fn handle_contact_force_event(
            &self,
            dt: f32,
            _bodies: &rapier2d::prelude::RigidBodySet,
            _colliders: &rapier2d::prelude::ColliderSet,
            contact_pair: &rapier2d::prelude::ContactPair,
            total_force_magnitude: f32,
        ) {
            let result =
                ContactForceEvent::from_contact_pair(dt, contact_pair, total_force_magnitude);
            let _ = self.force_sender.send(result);
        }
    }

    /// Let the body be added into this manager.
    pub fn add_body(
        m: &mut PhysicsManager,
        body: RigidBody,
        collider_v: Vec<Collider>,
    ) -> RigidBodyHandle {
        let body_handle = m.physics_engine.rigid_body_set.insert(body);

        for collider in collider_v {
            m.physics_engine.collider_set.insert_with_parent(
                collider,
                body_handle,
                &mut m.physics_engine.rigid_body_set,
            );
        }

        body_handle
    }
}

pub struct PhysicsManager {
    pub physics_engine: physics::PhysicsEngine,
    pub watcher: structs::Watcher,
    // pub on_event: Option<Rc<dyn Fn(SceneHandle, E)>>,
    // pub on_collision_event: Option<Rc<dyn Fn(SceneHandle, CollisionEvent)>>,
    // pub on_force_event: Option<Rc<dyn Fn(SceneHandle, ContactForceEvent)>>,
    // pub on_step: Option<Rc<dyn Fn(SceneHandle, u128)>>,
    // pub collision_event_rx: Receiver<CollisionEvent>,
    // pub force_event_rx: Receiver<ContactForceEvent>,
}

impl PhysicsManager {
    pub fn new(integration_parameters: IntegrationParameters) -> Self {
        let (collision_sender, _collision_event_rx) = channel();
        let (force_sender, _force_event_rx) = channel();
        let mut physics_engine = physics::PhysicsEngine::new(integration_parameters);
        physics_engine.set_event_handler(Box::new(inner::InnerEventHandler::new(
            collision_sender,
            force_sender,
        )));

        let watcher = structs::Watcher::new();
        Self {
            physics_engine,
            watcher,
            // on_event: None,
            // on_step: None,
            // on_collision_event: None,
            // on_force_event: None,
            // collision_event_rx,
            // force_event_rx,
        }
    }

    pub fn step(&mut self) {
        self.physics_engine.step();
    }
}

impl AsElementProvider for PhysicsManager {
    type H = RigidBodyHandle;

    fn update_element(&mut self, _: Self::H, props: &ViewProps) {
        match props.class.as_str() {
            _ => (),
        }
    }

    /// Let element be updated.
    fn delete_element(&mut self, h: RigidBodyHandle) {
        self.physics_engine.remove_rigid_body(h);
    }

    fn create_element(&mut self, _: u64, class: &str) -> RigidBodyHandle {
        match class {
            "ball" => inner::add_body(
                self,
                RigidBodyBuilder::fixed().build(),
                vec![ColliderBuilder::ball(1.0).build()],
            ),
            "quad" => inner::add_body(
                self,
                RigidBodyBuilder::fixed().build(),
                vec![ColliderBuilder::cuboid(0.5, 0.5).build()],
            ),
            _ => panic!(""),
        }
    }
}

pub struct RenderPass<'a> {
    vm: &'a mut VisionManager,
    output: SurfaceTexture,
    id_v: Vec<u64>,
}

impl<'a> RenderPass<'a> {
    pub fn push_element(&mut self, id: u64) {
        self.id_v.push(id);
    }

    pub fn render(self) -> err::Result<()> {
        let view = self
            .output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.vm
            .three_drawer
            .render(
                &self.vm.device,
                &self.vm.queue,
                &view,
                self.id_v
                    .iter()
                    .map(|id| self.vm.body_mp.get(id).unwrap().look.three_look.as_ref())
                    .filter(|op| op.is_some())
                    .map(|op| op.unwrap())
                    .collect(),
            )
            .change_context(err::Error::Other)?;

        // {
        //     let line_v = inner::gen_line_v(self.vm, &self.id_v);
        //     if !line_v.is_empty() {
        //         self.vm.ray_drawer.update_line_v(&self.vm.device, &line_v);

        //         // Draw ray tracing result to texture
        //         self.vm
        //             .ray_drawer
        //             .draw_ray_to_point_texture(&self.vm.device, &self.vm.queue);
        //     }

        //     // Let the points be drew to current surface.
        //     self.vm
        //         .surface_drawer
        //         .draw_point_to_surface(
        //             &self.vm.device,
        //             &self.vm.queue,
        //             &self.view,
        //             self.vm.ray_drawer.get_result_buffer(),
        //             self.vm.ray_drawer.get_size_buffer(),
        //         )
        //         .unwrap();
        // }

        // // Let the watcher be drew to current surface.
        // self.vm
        //     .light_drawer
        //     .draw_light_to_surface(
        //         &self.vm.device,
        //         &self.vm.queue,
        //         &self.view,
        //         self.vm.ray_drawer.get_watcher_buffer(),
        //         self.vm.ray_drawer.get_size_buffer(),
        //         &inner::gen_light_line_v(self.vm, &self.id_v),
        //     )
        //     .unwrap();

        self.output.present();

        Ok(())
    }
}

pub struct VisionManager {
    config: wgpu::SurfaceConfiguration,

    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub ray_drawer: drawer::RayDrawer,
    pub light_drawer: drawer::WathcerDrawer,
    pub three_drawer: drawer::ThreeDrawer,
    pub surface_drawer: drawer::SurfaceDrawer,

    pub body_mp: HashMap<u64, Body>,
}

impl VisionManager {
    pub fn new(
        ray_drawer: drawer::RayDrawer,
        light_drawer: drawer::WathcerDrawer,
        surface_drawer: drawer::SurfaceDrawer,
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let three_drawer = drawer::ThreeDrawer::new(
            &device,
            config.format,
            drawer::WGPU_OFFSET_M * Matrix4::new_perspective(1.0, PI * 0.6, 0.1, 500.0),
        );

        Self {
            ray_drawer,
            light_drawer,
            three_drawer,
            surface_drawer,
            device,
            queue,
            config,
            surface,
            body_mp: HashMap::new(),
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.ray_drawer.resize(&self.device, &self.queue, new_size);

            log::debug!("new_size = {new_size:?}");
        }
    }

    /// called => the result = a new render pass
    pub fn render_pass(&mut self, watcher: &Watcher) -> err::Result<RenderPass> {
        self.ray_drawer.update_watcher(&self.device, watcher);
        // Let the surface be drew.
        let output = self
            .surface
            .get_current_texture()
            .change_context(err::Error::Other)?;

        Ok(RenderPass {
            vm: self,
            output,
            id_v: Vec::new(),
        })
    }

    pub fn view_m(&self) -> &Matrix4<f32> {
        self.three_drawer.view_m()
    }

    pub fn view_m_mut(&mut self) -> &mut Matrix4<f32> {
        self.three_drawer.view_m_mut()
    }
}

impl AsElementProvider for VisionManager {
    type H = u64;

    fn create_element(&mut self, vnode_id: u64, class: &str) -> u64 {
        match class {
            "ball" => {
                log::debug!("create_element: create ball {vnode_id}");

                self.body_mp.insert(
                    vnode_id,
                    Body {
                        class: format!("ball"),
                        look: BodyLook {
                            ray_look: vec![RayLook {
                                shape: Shape::circle(),
                                shape_matrix: Matrix3::identity(),
                                color: Vector3::new(1.0, 1.0, 1.0),
                                light: 1.0,
                                roughness: 0.0,
                                seed: 0.0,
                                is_visible: true,
                            }],
                            light_look: vec![],
                            three_look: None,
                        },
                    },
                );
            }
            "light3" => {
                log::debug!("create_element: create light3 {vnode_id}");

                self.body_mp.insert(
                    vnode_id,
                    Body {
                        class: format!("light3"),
                        look: BodyLook {
                            ray_look: vec![],
                            light_look: vec![],
                            three_look: Some(ThreeLook::Light(Light {
                                color: vector![1.0, 1.0, 1.0, 1.0],
                                view: Matrix4::new_translation(&vector![0.0, 5.0, 0.0])
                                    * Matrix4::new_rotation(vector![PI * 0.25, 0.0, 0.0]),
                                proj: drawer::WGPU_OFFSET_M
                                    * Matrix4::new_orthographic(
                                        -10.0, 10.0, -10.0, 10.0, 0.0, 20.0,
                                    ),
                            })),
                        },
                    },
                );
            }
            "cube3" => {
                log::debug!("create_element: create cube3 {vnode_id}");

                self.body_mp.insert(
                    vnode_id,
                    Body {
                        class: format!("cube3"),
                        look: BodyLook {
                            ray_look: vec![],
                            light_look: vec![],
                            three_look: Some(ThreeLook::Body(Arc::new(
                                self.device.create_buffer_init(&BufferInitDescriptor {
                                    label: None,
                                    contents: bytemuck::cast_slice(
                                        drawer::structs::Body::cube(
                                            Matrix4::new_translation(&vector![0.0, 0.0, -3.0])
                                                * Matrix4::new_rotation(vector![
                                                    0.0,
                                                    -PI * 0.25,
                                                    0.0
                                                ]),
                                            vector![1.0, 1.0, 1.0, 1.0],
                                        )
                                        .vertex_v(),
                                    ),
                                    usage: BufferUsages::VERTEX,
                                }),
                            ))),
                        },
                    },
                );
            }
            "quad" => {
                self.body_mp.insert(
                    vnode_id,
                    Body {
                        class: format!("quad"),
                        look: BodyLook {
                            ray_look: vec![RayLook {
                                shape: Shape::quad(1.0, 1.0),
                                shape_matrix: Matrix3::identity(),
                                color: Vector3::new(1.0, 1.0, 1.0),
                                light: 0.0,
                                roughness: 0.0,
                                seed: 0.0,
                                is_visible: true,
                            }],
                            light_look: vec![],
                            three_look: None,
                        },
                    },
                );
            }
            _ => (),
        }

        vnode_id
    }

    fn delete_element(&mut self, id: u64) {
        self.body_mp.remove(&id);
    }

    fn update_element(&mut self, id: u64, props: &ViewProps) {
        if let Some(body) = self.body_mp.get_mut(&id) {
            match body.class.as_str() {
                "ball" => {
                    if let Some(radius) = props.props["$radius"][0].as_str() {
                        body.look.ray_look[0].shape_matrix =
                            Matrix3::new_scaling(radius.parse().unwrap());
                    }
                }
                "quad" => {
                    if let Some(height) = props.props["$height"][0].as_str() {
                        body.look.ray_look[0].shape_matrix =
                            Matrix3::new_nonuniform_scaling(&vector![1.0, height.parse().unwrap()]);
                    }
                }
                _ => (),
            }
        }
    }
}

pub struct InputProvider {}

impl InputProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl AsElementProvider for InputProvider {
    type H = u64;

    fn update_element(&mut self, id: Self::H, props: &ViewProps) {
        log::debug!("update_element: {id}")
    }

    fn delete_element(&mut self, id: Self::H) {
        log::debug!("delete_element: {id}")
    }

    fn create_element(&mut self, vnode_id: u64, class: &str) -> Self::H {
        log::debug!("create_element: vnode_id = {vnode_id}, class = {class}");

        vnode_id
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
