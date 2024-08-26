use std::{
    collections::HashMap, rc::Rc, sync::mpsc::{channel, Receiver}
};

use rapier2d::prelude::{CollisionEvent, ContactForceEvent};

use super::{handle::SceneHandle, physics, structs, Body};

pub struct Scene<D, E> {
    pub physics_engine: physics::PhysicsEngine,
    pub watcher: structs::Watcher,
    pub on_event: Option<Rc<dyn Fn(SceneHandle<D, E>, E)>>,
    pub on_collision_event: Option<Rc<dyn Fn(SceneHandle<D, E>, CollisionEvent)>>,
    pub on_force_event: Option<Rc<dyn Fn(SceneHandle<D, E>, ContactForceEvent)>>,
    pub on_step: Option<Rc<dyn Fn(SceneHandle<D, E>, u128)>>,
    pub collision_event_rx: Receiver<CollisionEvent>,
    pub force_event_rx: Receiver<ContactForceEvent>,
    pub body_index_mp: HashMap<String, HashMap<String, u64>>,
    pub body_mp: HashMap<u64, Body>,
}

impl<D, E> Scene<D, E> {
    pub fn new() -> Self {
        let (collision_sender, collision_event_rx) = channel();
        let (force_sender, force_event_rx) = channel();
        let mut physics_engine = physics::PhysicsEngine::new();
        physics_engine.set_event_handler(Box::new(inner::InnerEventHandler::new(
            collision_sender,
            force_sender,
        )));

        let watcher = structs::Watcher::new();
        Self {
            physics_engine,
            watcher,
            on_event: None,
            on_step: None,
            on_collision_event: None,
            on_force_event: None,
            collision_event_rx,
            force_event_rx,
            body_mp: HashMap::new(),
            body_index_mp: HashMap::new(),
        }
    }

    pub fn step(&mut self) {
        self.physics_engine.step();
    }
}

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
