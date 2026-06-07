use serde::{Deserialize, Serialize};

use crate::detection::Detection;
use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub confidence: f32,
    pub frame_count: usize,
}

impl Segment {
    pub fn new(start: f64, end: f64, confidence: f32) -> Self {
        Self {
            start,
            end,
            confidence,
            frame_count: 0,
        }
    }
    
    pub fn duration(&self) -> f64 {
        self.end - self.start
    }
    
    pub fn extend(&mut self, end: f64, confidence: f32) {
        self.end = end;
        self.confidence = self.confidence.max(confidence);
        self.frame_count += 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentBatch {
    pub input_file: String,
    pub segments: Vec<Segment>,
    pub total_duration: f64,
    pub human_duration: f64,
    pub compression_ratio: f64,
}

impl SegmentBatch {
    pub fn new(input_file: String) -> Self {
        Self {
            input_file,
            segments: Vec::new(),
            total_duration: 0.0,
            human_duration: 0.0,
            compression_ratio: 0.0,
        }
    }
    
    pub fn add_segment(&mut self, segment: Segment) {
        self.human_duration += segment.duration();
        self.segments.push(segment);
    }
    
    pub fn calculate_statistics(&mut self) {
        self.compression_ratio = if self.total_duration > 0.0 {
            self.human_duration / self.total_duration
        } else {
            0.0
        };
    }
    
    pub fn print_summary(&self) {
        println!("=== Segment Analysis ===");
        println!("Input file: {}", self.input_file);
        println!("Total duration: {:.2} seconds", self.total_duration);
        println!("Human activity: {:.2} seconds", self.human_duration);
        println!("Compression ratio: {:.2}%", self.compression_ratio * 100.0);
        println!("Number of segments: {}", self.segments.len());
        println!();
        
        for (i, seg) in self.segments.iter().enumerate() {
            println!("  Segment {}: {:.2}s - {:.2}s (duration: {:.2}s, confidence: {:.2})",
                     i + 1, seg.start, seg.end, seg.duration(), seg.confidence);
        }
    }
}

pub fn detect_segments(detections: &[Detection], config: &Config) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current_segment: Option<Segment> = None;
    
    for detection in detections {
        if detection.has_humans {
            match &mut current_segment {
                Some(segment) => {
                    segment.extend(detection.timestamp, detection.confidence);
                }
                None => {
                    let mut new_segment = Segment::new(
                        detection.timestamp, detection.timestamp, detection.confidence
                    );
                    new_segment.frame_count = 1;
                    current_segment = Some(new_segment);
                }
            }
        } else if let Some(segment) = current_segment.take() {
            if segment.duration() >= config.min_segment_duration {
                segments.push(segment);
            }
        }
    }
    
    if let Some(segment) = current_segment.take() {
        if segment.duration() >= config.min_segment_duration {
            segments.push(segment);
        }
    }
    
    segments
}