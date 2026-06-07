use anyhow::Result;

pub mod yolo;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub timestamp: f64,
    pub has_humans: bool,
    pub confidence: f32,
    pub bboxes: Vec<[f32; 4]>,
}

pub trait Detector {
    fn detect_frame(&mut self, frame: &[u8], width: i32, height: i32, timestamp: f64) -> Result<Detection>;
    fn cleanup(&mut self) -> Result<()>;
}