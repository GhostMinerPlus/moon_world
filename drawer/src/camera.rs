use std::f32::consts::FRAC_PI_2;

use nalgebra::{Matrix4, Point3, Vector3};

use crate::WGPU_OFFSET_M;

pub const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct CameraState {
    position: Point3<f32>,
    yaw: f32,
    pitch: f32,
}

impl CameraState {
    pub fn new<V: Into<Point3<f32>>, Y: Into<f32>, P: Into<f32>>(
        position: V,
        yaw: Y,
        pitch: P,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(
            &self.position,
            &Point3::new(
                self.position.x - self.yaw.sin(),
                self.position.y + self.pitch.sin(),
                self.position.z - self.yaw.cos(),
            ),
            &Vector3::new(0.0, 1.0, 0.0),
        )
    }

    pub fn position(&self) -> &Point3<f32> {
        &self.position
    }

    pub fn position_mut(&mut self) -> &mut Point3<f32> {
        &mut self.position
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn yaw_mut(&mut self) -> &mut f32 {
        &mut self.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn pitch_mut(&mut self) -> &mut f32 {
        &mut self.pitch
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<f32>>(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        WGPU_OFFSET_M * Matrix4::new_perspective(self.aspect, self.fovy, self.znear, self.zfar)
    }
}
