//! Help the crate be a video provider, a event handler or a shape builder.

use rapier3d::prelude::{Collider, GenericJoint};

pub mod shape;

pub struct BodyCollider {
    pub collider_v: Vec<Collider>,
}

pub struct Joint {
    pub body1: u64,
    pub body2: u64,
    pub joint: GenericJoint,
}
