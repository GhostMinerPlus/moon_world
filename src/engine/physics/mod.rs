use nalgebra::{Point2, Vector2};
use rapier2d::{parry::query::Ray, prelude::*};

pub struct PhysicsEngine {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,

    gravity: nalgebra::Matrix<
        f32,
        nalgebra::Const<2>,
        nalgebra::Const<1>,
        nalgebra::ArrayStorage<f32, 2, 1>,
    >,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    physics_hooks: (),
    event_handler: Box<dyn EventHandler>,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let gravity = vector![0.0, -9.81];
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();
        let physics_hooks = ();
        let event_handler = Box::new(());
        Self {
            rigid_body_set,
            collider_set,
            impulse_joint_set,
            multibody_joint_set,

            gravity,
            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            ccd_solver,
            query_pipeline,
            physics_hooks,
            event_handler,
        }
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &self.physics_hooks,
            self.event_handler.as_ref(),
        );
    }

    pub fn remove_rigid_body(&mut self, h: RigidBodyHandle) {
        self.rigid_body_set.remove(
            h,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }

    pub fn set_event_handler(&mut self, event_handler: Box<dyn EventHandler>) {
        self.event_handler = event_handler;
    }

    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: Real,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ColliderHandle, Real)> {
        self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            ray,
            max_toi,
            solid,
            filter,
        )
    }
}
