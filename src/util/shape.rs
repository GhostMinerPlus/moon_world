use std::f32::consts::PI;

use nalgebra::Point2;

#[derive(Clone)]
pub struct Shape {
    pub point_v: Vec<Point2<f32>>,
}

impl Shape {
    pub fn quad(w: f32, h: f32) -> Self {
        let point_v = vec![
            Point2::new(-w * 0.5, h * 0.5),
            Point2::new(-w * 0.5, -h * 0.5),
            Point2::new(w * 0.5, h * 0.5),
            Point2::new(w * 0.5, h * 0.5),
            Point2::new(-w * 0.5, h * 0.5),
        ];
        Self { point_v }
    }

    pub fn circle() -> Self {
        let point_v = (0..3601)
            .into_iter()
            .map(|i| {
                let angle = PI / 1800.0 * i as f32;
                Point2::new(angle.cos(), angle.sin())
            })
            .collect();
        Self { point_v }
    }

    pub fn none() -> Self {
        Self {
            point_v: Vec::new(),
        }
    }

    pub fn from_strip(point_v: Vec<Point2<f32>>) -> Self {
        Self { point_v }
    }
}
