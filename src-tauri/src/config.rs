use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub yolo_confidence: f32,
    pub min_segment_duration: f64,
    pub merge_gap_seconds: f64,
    pub sample_rate_frames: usize,
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            yolo_confidence: 0.35,
            min_segment_duration: 2.0,
            merge_gap_seconds: 3.0,
            sample_rate_frames: 5,
            output_dir: PathBuf::from("./output"),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self, anyhow::Error> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }
    
    pub fn save(&self, path: &str) -> Result<(), anyhow::Error> {
        let contents = serde_yaml::to_string(&self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}