//! Help the crate be a video provider, a event handler or a shape builder.

use std::sync::Arc;

use nalgebra::{Matrix3, Matrix4, Vector3, Vector4};
use rapier2d::prelude::{Collider, GenericJoint};

pub mod shape;

pub struct BodyLook {
    pub ray_look: Vec<RayLook>,
    pub light_look: Vec<LightLook>,
    pub three_look: Option<ThreeLook>,
}

/// gotten with body => the result = buffer of body
pub enum ThreeLook {
    Body(Arc<wgpu::Buffer>),
    Light(Light)
}

pub struct Light {
    pub color: Vector4<f32>,
    pub matrix: Matrix4<f32>,
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
    pub life_step_op: Option<u64>,
    pub matrix: Matrix3<f32>,
}

pub struct Joint {
    pub body1: u64,
    pub body2: u64,
    pub joint: GenericJoint,
}
