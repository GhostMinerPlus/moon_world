use std::{collections::HashMap, rc::Rc};

use rapier2d::prelude::{Collider, ColliderHandle, CollisionEvent, ContactForceEvent, QueryFilter, Ray, Real, RigidBody, RigidBodyHandle};

use super::{Body, BodyBuilder, Engine, Joint};

/// Scene
pub struct SceneHandle<'a, D, E> {
    pub(crate) engine: &'a mut Engine<D, E>,
    pub(crate) scene_id: u64,
}

impl<'a, D, E> SceneHandle<'a, D, E> {
    /// Get scene id.
    pub fn scene_id(&self) -> u64 {
        return self.scene_id;
    }

    //// Add a body into this scene.
    pub fn add_body(&mut self, mut body: BodyBuilder) -> u64 {
        let body_id = self.engine.unique_id;
        self.engine.unique_id += 1;
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        body.rigid.user_data = body_id as u128;
        let body_handle = scene.physics_engine.rigid_body_set.insert(body.rigid);
        self.engine.body_mp.insert(
            body_id,
            Body {
                class: body.class.clone(),
                name: body.name.clone(),
                look: body.look,
                rigid: body_handle,
                life_step_op: body.life_step_op,
            },
        );
        match self.engine.body_index_mp.get_mut(&body.class) {
            Some(mp) => {
                mp.insert(body.name, body_id);
            }
            None => {
                let mut mp = HashMap::new();
                mp.insert(body.name, body_id);
                self.engine.body_index_mp.insert(body.class.clone(), mp);
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

    /// Add joint for this scene.
    pub fn add_joint(&mut self, mut joint: Joint) -> u64 {
        let joint_id = self.engine.unique_id;
        self.engine.unique_id += 1;
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        let body1 = &self.engine.body_mp[&joint.body1];
        let body2 = &self.engine.body_mp[&joint.body2];
        joint.joint.user_data = joint_id as u128;
        scene
            .physics_engine
            .impulse_joint_set
            .insert(body1.rigid, body2.rigid, joint.joint, true);
        joint_id
    }

    /// Set window event listener for this scene.
    pub fn set_event_listener(&mut self, listener: Rc<dyn Fn(SceneHandle<D, E>, E)>) {
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        scene.on_event = Some(listener);
    }

    /// Set step listener for this scene.
    pub fn set_step_listener(&mut self, listener: Rc<dyn Fn(SceneHandle<D, E>, u128)>) {
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        scene.on_step = Some(listener);
    }

    /// Bind wathcher to a body.
    pub fn bind_watcher(&mut self, body_id: u64) {
        self.engine.watcher_binding_body_id = body_id
    }

    /// Set collision event handler for this scene.
    pub fn set_collision_event_handler(
        &mut self,
        event_handler: Rc<dyn Fn(SceneHandle<D, E>, CollisionEvent)>,
    ) {
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        scene.on_collision_event = Some(event_handler);
    }

    /// Set force event handler for this scene.
    pub fn set_force_event_handler(
        &mut self,
        event_handler: Rc<dyn Fn(SceneHandle<D, E>, ContactForceEvent)>,
    ) {
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        scene.on_force_event = Some(event_handler);
    }

    /// Get the engine.
    pub fn get_engine(&self) -> &Engine<D, E> {
        &self.engine
    }

    /// Get the engine.
    pub fn get_engine_mut(&mut self) -> &mut Engine<D, E> {
        &mut self.engine
    }

    /// Get the body id of specified collider.
    pub fn get_body_id_of_collider(&self, ch: ColliderHandle) -> u64 {
        let scene = self.engine.scene_mp.get(&self.scene_id).unwrap();
        let rigid_body = &scene.physics_engine.rigid_body_set
            [scene.physics_engine.collider_set[ch].parent().unwrap()];
        rigid_body.user_data as u64
    }

    /// Get body by id.
    pub fn get_body_mut(&mut self, id: &u64) -> Option<&mut Body> {
        self.engine.body_mp.get_mut(id)
    }

    /// Get body by id.
    pub fn get_body(&self, id: &u64) -> Option<&Body> {
        self.engine.body_mp.get(id)
    }

    /// Get body ids by class.
    pub fn get_body_id_v_by_class(&self, class: &str) -> Vec<u64> {
        match self.engine.body_index_mp.get(class) {
            Some(mp) => mp.iter().map(|(_, v)| *v).collect(),
            None => Vec::new(),
        }
    }

    /// Get body id by its class and name.
    pub fn get_body_id_by_class_name(&self, class: &str, name: &str) -> Option<u64> {
        self.engine
            .body_index_mp
            .get(class)?
            .get(name)
            .map(|id| *id)
    }

    /// Get the collider by its handle
    pub fn get_collider(&self, h: ColliderHandle) -> Option<&Collider> {
        let scene = self.engine.scene_mp.get(&self.scene_id).unwrap();
        scene.physics_engine.collider_set.get(h)
    }

    /// Find the closest intersection between a ray and a set of collider.
    ///
    /// # Parameters
    /// * `ray`: the ray to cast.
    /// * `max_toi`: the maximum time-of-impact that can be reported by this cast. This effectively
    ///   limits the length of the ray to `ray.dir.norm() * max_toi`. Use `Real::MAX` for an unbounded ray.
    /// * `solid`: if this is `true` an impact at time 0.0 (i.e. at the ray origin) is returned if
    ///            it starts inside of a shape. If this `false` then the ray will hit the shape's boundary
    ///            even if its starts inside of it.
    /// * `filter`: set of rules used to determine which collider is taken into account by this scene query.
    /// # Return
    /// * `None`: if not found.
    /// * `Some((ColliderHandle, Real))`: the collider and the distance.
    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: Real,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ColliderHandle, Real)> {
        let scene = self.engine.scene_mp.get(&self.scene_id).unwrap();
        scene.physics_engine.cast_ray(ray, max_toi, solid, filter)
    }

    pub fn get_rigid_body(&self, h: RigidBodyHandle) -> Option<&RigidBody> {
        let scene = self.engine.scene_mp.get(&self.scene_id).unwrap();
        scene.physics_engine.rigid_body_set.get(h)
    }

    pub fn get_rigid_body_mut(&mut self, h: RigidBodyHandle) -> Option<&mut RigidBody> {
        let scene = self.engine.scene_mp.get_mut(&self.scene_id).unwrap();
        scene.physics_engine.rigid_body_set.get_mut(h)
    }
}
