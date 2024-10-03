use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver},
};

use rapier2d::prelude::{CollisionEvent, ContactForceEvent, IntegrationParameters};
use rodio::{cpal::FromSample, OutputStream, OutputStreamHandle, Sample, Sink, Source};

use super::{drawer, physics, structs, Body};

mod inner {
    use std::sync::mpsc::Sender;

    use rapier2d::prelude::{ContactForceEvent, EventHandler};

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
    pub body_index_mp: HashMap<String, HashMap<String, u64>>,
    pub body_mp: HashMap<u64, Body>,
}

impl PhysicsManager {
    pub fn new(integration_parameters: IntegrationParameters) -> Self {
        let (collision_sender, collision_event_rx) = channel();
        let (force_sender, force_event_rx) = channel();
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
            body_mp: HashMap::new(),
            body_index_mp: HashMap::new(),
        }
    }

    pub fn step(&mut self) {
        self.physics_engine.step();
    }

    pub fn remove_body(&mut self, id: &u64) {
        if let Some(body) = self.body_mp.remove(id) {
            if let Some(set) = self.body_index_mp.get_mut(&body.class) {
                set.remove(&body.name);
            }
            self.physics_engine.remove_rigid_body(body.rigid);
        }
    }

    /// Get body id by its class and name.
    pub fn get_body_id_by_class_name(&self, class: &str, name: &str) -> Option<u64> {
        self.body_index_mp.get(class)?.get(name).map(|id| *id)
    }
}

pub struct VisionManager {
    pub ray_drawer: drawer::RayDrawer,
    pub light_drawer: drawer::WathcerDrawer,
    pub surface_drawer: drawer::SurfaceDrawer,

    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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
        Self {
            ray_drawer,
            light_drawer,
            surface_drawer,
            device,
            queue,
            config,
            surface,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.ray_drawer.resize(&self.device, &self.queue, new_size);
        }
    }
}

pub struct AudioManager {}

impl AudioManager {
    pub fn new() -> Self {
        Self {}
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

impl AudioManager {
    /// Mix a sound into this engine.
    pub fn mix_sound<S>(&self, source: S) -> Sink
    where
        S: Source + Send + 'static,
        f32: FromSample<S::Item>,
        S::Item: Sample + Send,
    {
        let (_output_stream, output_stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&output_stream_handle).unwrap();
        sink.append(source);
        sink
    }
}
