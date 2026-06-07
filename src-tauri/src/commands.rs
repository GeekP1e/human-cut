use humancut_lib::config::Config;
use humancut_lib::processing::process::process_video;
use humancut_lib::demo::demo::run_demo;
use std::path::PathBuf;
use std::string::String;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProcessConfig {
    pub input: String,
    pub output: String,
    pub confidence: f32,
    pub min_duration: f64,
    pub merge_gap: f64,
    pub sample_rate: usize,
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn download_model_cmd(modelType: String) -> Result<String, String> {
    println!("model_type {}", modelType);
    let model_type = modelType.clone();
    tokio::task::spawn_blocking(move || {
        humancut_lib::processing::process::download_yolo_model(&model_type)
            .map(|p| p.display().to_string())
            .map_err(|e: anyhow::Error| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn process_video_cmd(cfg: ProcessConfig) -> Result<String, String> {
    let mut config = Config::default();
    config.yolo_confidence = cfg.confidence;
    config.min_segment_duration = cfg.min_duration;
    config.merge_gap_seconds = cfg.merge_gap;
    config.sample_rate_frames = cfg.sample_rate;
    config.output_dir = PathBuf::from(&cfg.output);

    let input_path = PathBuf::from(cfg.input);

    tokio::task::spawn_blocking(move || {
        process_video(&input_path, &config)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|_| "Done".to_string())
    .map_err(|e: anyhow::Error| e.to_string())
}


#[tauri::command]
pub async fn run_demo_cmd(output: String) -> Result<String, String> {
    let output_path = PathBuf::from(output);
    tokio::task::spawn_blocking(move || {
        run_demo(&output_path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|_| "Done".to_string())
    .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn get_available_models() -> Result<Vec<String>, String> {
    let models_dir = std::path::Path::new("./models");

    if !models_dir.exists() {
        return Ok(vec![]);
    }

    let models = std::fs::read_dir(models_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("onnx"))
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    Ok(models)
}

#[tauri::command]
pub async fn get_output_videos(output_dir: String, video_name: String) -> Result<Vec<String>, String> {
    let dir = std::path::Path::new(&output_dir).join(&video_name);
    
    if !dir.exists() {
        return Ok(vec![]);
    }

    let videos = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("mp4" | "mkv" | "avi" | "mov")
        ))
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    Ok(videos)
}

#[tauri::command]
pub async fn open_video_file(path: String) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}