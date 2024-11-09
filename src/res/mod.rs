use std::{
    collections::HashMap,
    f32::consts::PI,
    sync::{mpsc::channel, Arc},
};

use drawer::{Body, Light, ThreeLook};
use error_stack::ResultExt;
use nalgebra::{vector, Matrix4};
use rapier2d::prelude::{
    ColliderBuilder, IntegrationParameters, RigidBodyBuilder, RigidBodyHandle,
};
use view_manager::{AsElementProvider, ViewProps};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, SurfaceTexture,
};

use crate::err;

use super::physics;

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

        Self {
            physics_engine,
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
                    .map(|id| self.vm.body_mp.get(id))
                    .filter(|op| op.is_some())
                    .map(|op| op.unwrap())
                    .collect(),
            )
            .change_context(err::Error::Other)?;

        self.output.present();

        Ok(())
    }
}

pub struct VisionManager {
    config: wgpu::SurfaceConfiguration,

    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub three_drawer: drawer::ThreeDrawer,

    pub body_mp: HashMap<u64, ThreeLook>,
}

impl VisionManager {
    pub fn new(
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
            three_drawer,
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

            log::debug!("new_size = {new_size:?}");
        }
    }

    /// called => the result = a new render pass
    pub fn render_pass(&mut self) -> err::Result<RenderPass> {
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
            "light3" => {
                log::debug!("create_element: create light3 {vnode_id}");

                self.body_mp.insert(
                    vnode_id,
                    ThreeLook::Light(Light {
                        color: vector![1.0, 1.0, 1.0, 1.0],
                        view: Matrix4::new_translation(&vector![0.0, 5.0, 0.0])
                            * Matrix4::new_rotation(vector![PI * 0.25, 0.0, 0.0]),
                        proj: drawer::WGPU_OFFSET_M
                            * Matrix4::new_orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 20.0),
                    }),
                );
            }
            "cube3" => {
                log::debug!("create_element: create cube3 {vnode_id}");

                self.body_mp.insert(
                    vnode_id,
                    ThreeLook::Body(Body {
                        model_m: Matrix4::new_translation(&vector![0.0, 0.0, -3.0])
                            * Matrix4::new_rotation(vector![0.0, -PI * 0.25, 0.0]),
                        buf: Arc::new(
                            self.device.create_buffer_init(&BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(
                                    drawer::structs::Point3InputArray::cube(vector![
                                        1.0, 1.0, 1.0, 1.0
                                    ])
                                    .vertex_v(),
                                ),
                                usage: BufferUsages::VERTEX,
                            }),
                        ),
                    }),
                );
            }
            _ => (),
        }

        vnode_id
    }

    fn delete_element(&mut self, id: u64) {
        self.body_mp.remove(&id);
    }

    fn update_element(&mut self, id: u64, view_props: &ViewProps) {
        if let Some(_) = self.body_mp.get_mut(&id) {
            match view_props.class.as_str() {
                // "cube3" => {}
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

    fn update_element(&mut self, id: Self::H, _props: &ViewProps) {
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
