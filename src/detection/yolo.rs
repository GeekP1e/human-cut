use anyhow::{Result, anyhow};

use super::{Detector, Detection};
use ort::{
    session::Session,
    value::TensorRef,
};
use ndarray::Array4;
use std::path::Path;

const YOLO_INPUT_SIZE: usize = 640;

#[derive(Clone)]
struct BoundingBox {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    confidence: f32,
}

pub struct YoloDetector {
    session: Session,
    confidence_threshold: f32,
    nms_threshold: f32,
}

impl YoloDetector {
    pub fn new(confidence_threshold: f32, model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            anyhow::bail!("YOLO model not found at: {}", model_path.display());
        }
        
        println!("Loading YOLO model from: {}", model_path.display());
        let session = Session::builder()?.commit_from_file(model_path)?;
        println!("✅ Model loaded successfully!");

        Ok(Self {
            session,
            confidence_threshold,
            nms_threshold: 0.45,
        })
    }

    fn preprocess_frame(&self, frame: &[u8], width: i32, height: i32) -> Result<Array4<f32>> {
        let img = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(
            width as u32,
            height as u32,
            frame.to_vec(),
        )
        .ok_or_else(|| anyhow!("Failed to create image from frame"))?;

        let resized = image::imageops::resize(
            &img,
            YOLO_INPUT_SIZE as u32,
            YOLO_INPUT_SIZE as u32,
            image::imageops::FilterType::Triangle,
        );

        let mut tensor = Array4::zeros((1, 3, YOLO_INPUT_SIZE, YOLO_INPUT_SIZE));

        for y in 0..YOLO_INPUT_SIZE {
            for x in 0..YOLO_INPUT_SIZE {
                let pixel = resized.get_pixel(x as u32, y as u32);
                tensor[[0, 0, y, x]] = pixel[0] as f32 / 255.0;
                tensor[[0, 1, y, x]] = pixel[1] as f32 / 255.0;
                tensor[[0, 2, y, x]] = pixel[2] as f32 / 255.0;
            }
        }

        Ok(tensor)
    }

    fn postprocess_outputs(
        &self,
        data: &[f32],
        num_detections: usize,
        _num_channels: usize,
        img_width: i32,
        img_height: i32,
    ) -> Vec<BoundingBox> {
        let mut detections = Vec::new();

        let scale_x = img_width as f32 / YOLO_INPUT_SIZE as f32;
        let scale_y = img_height as f32 / YOLO_INPUT_SIZE as f32;

        for i in 0..num_detections {
            let person_score = data[4 * num_detections + i];

            if person_score < self.confidence_threshold {
                continue;
            }

            let cx = data[0 * num_detections + i];
            let cy = data[1 * num_detections + i];
            let w  = data[2 * num_detections + i];
            let h  = data[3 * num_detections + i];

            let x = ((cx - w / 2.0) * scale_x).max(0.0);
            let y = ((cy - h / 2.0) * scale_y).max(0.0);
            let bw = (w * scale_x).min(img_width as f32 - x);
            let bh = (h * scale_y).min(img_height as f32 - y);

            detections.push(BoundingBox {
                x,
                y,
                width: bw,
                height: bh,
                confidence: person_score,
            });
        }

        self.apply_nms(detections)
    }

    fn apply_nms(&self, mut detections: Vec<BoundingBox>) -> Vec<BoundingBox> {
        if detections.is_empty() {
            return Vec::new();
        }

        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let mut keep: Vec<BoundingBox> = Vec::new();

        while !detections.is_empty() {
            let best = detections.remove(0);
            detections.retain(|box2| Self::calculate_iou(&best, box2) < self.nms_threshold);
            keep.push(best);
        }

        keep
    }

    fn calculate_iou(box1: &BoundingBox, box2: &BoundingBox) -> f32 {
        let x1 = box1.x.max(box2.x);
        let y1 = box1.y.max(box2.y);
        let x2 = (box1.x + box1.width).min(box2.x + box2.width);
        let y2 = (box1.y + box1.height).min(box2.y + box2.height);

        if x2 < x1 || y2 < y1 {
            return 0.0;
        }

        let intersection = (x2 - x1) * (y2 - y1);
        let area1 = box1.width * box1.height;
        let area2 = box2.width * box2.height;
        let union = area1 + area2 - intersection;

        if union <= 0.0 { 0.0 } else { intersection / union }
    }
}

impl Detector for YoloDetector {
    fn detect_frame(
        &mut self,
        frame: &[u8],
        width: i32,
        height: i32,
        timestamp: f64,
    ) -> Result<Detection> {
        let input_tensor = self.preprocess_frame(frame, width, height)?;

        let (num_channels, num_detections, data_owned) = {
            let outputs = self.session.run(
                ort::inputs![TensorRef::from_array_view(&input_tensor)?]
            )?;

            let (_name, output) = outputs.iter().next()
                .ok_or_else(|| anyhow!("No model output"))?;

            let tensor = output.try_extract_tensor::<f32>()?;
            let (shape, data) = &tensor;
            let shape_vec: Vec<usize> = shape.as_ref().iter().map(|&d| d as usize).collect();

            (shape_vec[1], shape_vec[2], data.to_vec())
        };

        let bboxes = self.postprocess_outputs(&data_owned, num_detections, num_channels, width, height);

        let max_confidence = bboxes.iter()
            .map(|b| b.confidence)
            .fold(0.0f32, f32::max);

        let has_humans = !bboxes.is_empty();

        let bbox_arrays: Vec<[f32; 4]> = bboxes.iter()
            .map(|b| [b.x, b.y, b.width, b.height])
            .collect();

        Ok(Detection {
            timestamp,
            has_humans,
            confidence: max_confidence,
            bboxes: bbox_arrays,
        })
    }

    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
}
