use anyhow::Result;
use log::{info, warn};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::config::Config;
use crate::detection::{Detector};
use crate::detection::yolo::YoloDetector;
use crate::processing::segment::{SegmentBatch};
use crate::processing::merger::SegmentMerger;
use crate::processing::exporter::VideoExporter;
use crate::processing::reader::VideoReader;
use crate::processing::segment::detect_segments;

pub fn download_yolo_model(model_type: &str) -> Result<PathBuf> {
    let models_dir = Path::new("./models");
    std::fs::create_dir_all(models_dir)?;

    let (model_filename, model_url) = match model_type {
        "nano" => (
            "yolov8n.onnx",
            "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolov8n.onnx",
        ),
        "small" => (
            "yolov8s.onnx",
            "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolov8s.onnx",
        ),
        "medium" => (
            "yolov8m.onnx",
            "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolov8m.onnx",
        ),
        _ => anyhow::bail!("Unknown model type: {}. Use: nano, small, medium", model_type),
    };
    let model_path = models_dir.join(model_filename);

    if model_path.exists() && model_path.metadata()?.len() > 0 {
        println!("✅ Model already exists: {}", model_path.display());
        return Ok(model_path);
    }

    println!("📥 Downloading YOLOv8 {} model...", model_type);
    let mut response = reqwest::blocking::get(model_url)?.error_for_status()?;
    let expected_size = response.content_length();
    let temp_path = model_path.with_extension("onnx.download");
    if temp_path.exists() {
        std::fs::remove_file(&temp_path)?;
    }

    let mut file = std::fs::File::create(&temp_path)?;
    let downloaded = std::io::copy(&mut response, &mut file)?;
    file.flush()?;
    file.sync_all()?;

    if downloaded == 0 {
        let _ = std::fs::remove_file(&temp_path);
        anyhow::bail!("Downloaded model is empty. Please download manually into ./models/");
    }

    if let Some(expected_size) = expected_size {
        if downloaded != expected_size {
            let _ = std::fs::remove_file(&temp_path);
            anyhow::bail!(
                "Incomplete model download: got {} of {} bytes",
                downloaded,
                expected_size
            );
        }
    }

    if model_path.exists() {
        std::fs::remove_file(&model_path)?;
    }
    std::fs::rename(&temp_path, &model_path)?;
    println!(
        "✅ Model saved: {} ({:.2} MB)",
        model_path.display(),
        downloaded as f64 / (1024.0 * 1024.0)
    );

    Ok(model_path)
}


pub fn process_video(input_path: &Path, config: &Config) -> Result<()> {
    if !input_path.exists() {
        anyhow::bail!("Input file does not exist: {:?}", input_path);
    }

    println!("yolo_confidence {}", &config.yolo_confidence);
    println!("min_segment_duration {}", &config.min_segment_duration);
    println!("merge_gap_seconds {}", &config.merge_gap_seconds);
    println!("sample_rate_frames {}", &config.sample_rate_frames);
    println!("output_dir {}", &config.output_dir.display());
    

    info!("Processing video: {:?}", input_path.file_name().unwrap());
    info!("Confidence: {}, Sample rate: {}", 
          config.yolo_confidence, config.sample_rate_frames);
    
    let start_time = Instant::now();
    
    println!("Looking for model...");
    let model_path = download_yolo_model("nano")?;
    println!("✅ Model found: {}", model_path.display());
    let mut detector = YoloDetector::new(config.yolo_confidence, &model_path)?;
    
    let mut video_reader = VideoReader::new(input_path)?;
    let detections = video_reader.process_with_detector(&mut detector, config)?;
    
    if detections.is_empty() {
        warn!("No detections recorded");
        detector.cleanup()?;
        return Ok(());
    }
    
    let mut segments = detect_segments(&detections, config);
    
    let merger = SegmentMerger::new(config.merge_gap_seconds);
    segments = merger.merge(&segments);
    segments = merger.filter_by_duration(&segments, config.min_segment_duration);
    
    let mut batch = SegmentBatch::new(input_path.to_string_lossy().to_string());
    batch.total_duration = video_reader.get_duration();
    for segment in &segments {
        batch.add_segment(segment.clone());
    }
    batch.calculate_statistics();
    batch.print_summary();
    
    if !batch.segments.is_empty() {
        let exporter = VideoExporter::new(config.output_dir.clone());
        let prefix = format!("human_{}", input_path.file_stem().unwrap().to_string_lossy());
        
        println!("\nExporting segments...");
        let output_paths = exporter.export_batch(input_path, &batch.segments, &prefix)?;
        
        let elapsed = start_time.elapsed();
        println!("\n✅ Successfully exported {} segments in {:.1}s", output_paths.len(), elapsed.as_secs_f64());
        
        let original_size = std::fs::metadata(input_path)?.len();
        let output_size: u64 = output_paths.iter()
            .map(|p| p.metadata().map(|m| m.len()).unwrap_or(0))
            .sum();
        
        let saved = original_size as f64 - output_size as f64;
        let percent = (output_size as f64 / original_size as f64) * 100.0;
        
        println!("Original size: {:.2} MB", original_size as f64 / (1024.0 * 1024.0));
        println!("Output size: {:.2} MB ({:.1}% of original)", output_size as f64 / (1024.0 * 1024.0), percent);
        println!("Saved {:.2} MB of disk space!", saved / (1024.0 * 1024.0));
        
    } else {
        println!("\n⚠️  No human activity detected in the video.");
        println!("   Try lowering confidence threshold (e.g., --confidence 0.3)");
    }
    
    detector.cleanup()?;
    Ok(())
}
