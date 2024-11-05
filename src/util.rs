//! Help the crate be a video provider, a event handler or a shape builder.

use drawer::ThreeLook;
use nalgebra::{Matrix3, Vector3};
use rapier2d::prelude::{Collider, GenericJoint};

pub mod shape;

pub struct BodyLook {
    pub ray_look: Vec<RayLook>,
    pub light_look: Vec<LightLook>,
    pub three_look: Option<ThreeLook>,
}

pub struct LightLook {
    pub shape: shape::Shape,
    pub shape_matrix: Matrix3<f32>,
    pub color: Vector3<f32>,
    pub is_visible: bool,
}

pub struct RayLook {
    pub shape: shape::Shape,
    pub shape_matrix: Matrix3<f32>,
    pub color: Vector3<f32>,
    pub light: f32,
    pub roughness: f32,
    pub seed: f32,
    pub is_visible: bool,
}

pub struct BodyCollider {
    pub collider_v: Vec<Collider>,
}

pub struct Body {
    pub class: String,
    pub look: BodyLook,
}

pub struct Joint {
    pub body1: u64,
    pub body2: u64,
    pub joint: GenericJoint,
}
