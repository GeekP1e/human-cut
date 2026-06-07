use anyhow::{Result, anyhow};
use log::{info, error};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::config::Config;
use crate::detection::{Detector, Detection};

pub struct VideoReader {
    path: PathBuf,
    fps: f64,
    width: i32,
    height: i32,
    duration: f64,
}

impl VideoReader {
    pub fn new(path: &Path) -> Result<Self> {
        use std::process::Command;
        
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_streams")
            .arg("-show_format")
            .arg(path.to_str().unwrap())
            .output()?;
        
        let info: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        
        let video_stream = info["streams"]
            .as_array()
            .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))
            .ok_or_else(|| anyhow!("No video stream found"))?;
        
        let fps_str = video_stream["r_frame_rate"].as_str().unwrap_or("30/1");
        let fps: f64 = fps_str
            .split_once('/')
            .map(|(num, den)| {
                let num: f64 = num.parse().unwrap_or(30.0);
                let den: f64 = den.parse().unwrap_or(1.0);
                num / den
            })
            .unwrap_or_else(|| fps_str.parse().unwrap_or(30.0));
        
        let width = video_stream["width"].as_i64().unwrap_or(1920) as i32;
        let height = video_stream["height"].as_i64().unwrap_or(1080) as i32;
        
        let duration = [
            &video_stream["duration"],
            &info["format"]["duration"],
        ]
        .iter()
        .find_map(|value| value.as_str()?.parse::<f64>().ok())
        .unwrap_or(0.0);
        
        info!("Video info: {}x{} at {:.2} fps, duration: {:.2}s", 
              width, height, fps, duration);
        
        Ok(Self {
            path: path.to_path_buf(),
            fps,
            width,
            height,
            duration,
        })
    }
    
    pub fn get_duration(&self) -> f64 {
        self.duration
    }
    
    pub fn process_with_detector(&mut self, detector: &mut dyn Detector, config: &Config) -> Result<Vec<Detection>> {
        info!("Processing video: {:?}", self.path);
        let start_time = Instant::now();
        
        let mut detections = Vec::new();
        
        use std::process::{Command, Stdio};
        use std::io::{Read, ErrorKind};
        
        let mut child = Command::new("ffmpeg")
            .arg("-i")
            .arg(self.path.to_str().unwrap())
            .arg("-vf")
            .arg(&format!("fps={}", self.fps))
            .arg("-pix_fmt")
            .arg("rgb24")
            .arg("-f")
            .arg("rawvideo")
            .arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        
        let stdout = child.stdout.as_mut().ok_or_else(|| anyhow!("Failed to get stdout"))?;
        
        let frame_size = (self.width * self.height * 3) as usize;
        let mut frame_buffer = vec![0u8; frame_size];
        let mut frame_index = 0;
        
        let sample_rate = config.sample_rate_frames.max(1);
        
        loop {
            match stdout.read_exact(&mut frame_buffer) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            
            let timestamp = frame_index as f64 / self.fps;
            frame_index += 1;
            
            if frame_index % sample_rate != 0 {
                continue;
            }
            
            match detector.detect_frame(&frame_buffer, self.width, self.height, timestamp) {
                Ok(detection) => {
                    let has_humans = detection.has_humans;
                    let confidence = detection.confidence;
                    let bboxes_len = detection.bboxes.len();
                    detections.push(detection);
                    if has_humans {
                        info!("Humans at {:.2}s (confidence: {:.2}, boxes: {})", 
                            timestamp, confidence, bboxes_len);
                    }
                }
                Err(e) => {
                    error!("Detection failed at {:.2}s: {}", timestamp, e);
                }
            }
            
            let progress_interval = (self.fps as usize * 5).max(1);
            if frame_index as usize % progress_interval == 0 && self.duration > 0.0 {
                let progress = (timestamp / self.duration) * 100.0;
                info!("Progress: {:.1}% ({}s / {}s)", progress, timestamp as i32, self.duration as i32);
            }
        }
        
        child.wait()?;
        
        let elapsed = start_time.elapsed();
        info!("Video processing completed in {:.2}s", elapsed.as_secs_f64());
        
        Ok(detections)
    }
}

