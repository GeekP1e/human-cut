mod config;
mod detection;
mod processing;
mod demo;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use env_logger;
use log::{info, warn};
use std::path::PathBuf;

use config::Config;
use processing::process::{download_yolo_model, process_video};
use demo::demo::run_demo;


#[derive(Parser)]
#[command(name = "humancut")]
#[command(about = "Extract video segments containing humans using YOLO", long_about = None)]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(value_name = "INPUT", default_value = "./videos")]
        input: PathBuf,
        
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
        
        #[arg(short, long)]
        config: Option<String>,
        
        #[arg(long, default_value = "1.0")]
        min_duration: f64,
        
        #[arg(long, default_value = "5.0")]
        merge_gap: f64,
        
        #[arg(long, default_value = "0.5")]
        confidence: Option<f32>,
        
        #[arg(long, default_value = "5")]
        sample_rate: Option<usize>,
    },
    
    GenerateConfig {
        #[arg(default_value = "config.yaml")]
        output: String,
    },
    
    DownloadModel {
        #[arg(default_value = "nano")]
        model_type: String,
    },

    Demo {
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },
}


fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Extract {
            input,
            output,
            config,
            min_duration,
            merge_gap,
            confidence,
            sample_rate,
        } => {
            let mut cfg = if let Some(config_path) = config {
                Config::load(&config_path)?
            } else {
                Config::default()
            };
            
            cfg.min_segment_duration = min_duration;
            cfg.merge_gap_seconds = merge_gap;
            cfg.output_dir = output;
            
            if let Some(conf) = confidence {
                cfg.yolo_confidence = conf;
            }
            
            if let Some(rate) = sample_rate {
                cfg.sample_rate_frames = rate;
            }
            cfg.sample_rate_frames = cfg.sample_rate_frames.max(1);

            if input == PathBuf::from("./videos") {
                let entries: Vec<PathBuf> = std::fs::read_dir(&input)
                    .map_err(|_| anyhow!("Folder ./videos not found"))?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| matches!(
                        p.extension().and_then(|s| s.to_str()),
                        Some("mp4" | "mkv" | "avi" | "mov")
                    ))
                    .collect();

                if entries.is_empty() {
                    warn!("No video files found in ./videos");
                    println!("⚠️  Put your videos into the ./videos folder and try again.");
                } else {
                    for video in entries {
                        info!("Processing: {:?}", video);
                        process_video(&video, &cfg)?;
                    }
                }
            } else {
                if input.is_dir() {
                    println!("⚠️  You provided a folder path, not a file.");
                    println!("   Please either:");
                    println!("   1. Provide a direct path to a video file:");
                    println!("      cargo run -- extract ./myfolder/video.mp4");
                    println!("   2. Place your video into ./videos/ and run without arguments:");
                    println!("      cargo run -- extract");
                } else {
                    process_video(&input, &cfg)?;
                }
            }
        }
        
        Commands::GenerateConfig { output } => {
            let cfg = Config::default();
            cfg.save(&output)?;
            println!("✅ Config template saved to {}", output);
            println!("\n📝 Edit this file to adjust detection parameters:");
            println!("   - yolo_confidence: YOLO confidence (0.3-0.7 recommended)");
            println!("   - min_segment_duration: minimum clip length in seconds");
            println!("   - merge_gap_seconds: merge clips with smaller gaps");
            println!("   - sample_rate_frames: process every Nth frame (1-30)");
        }
        
        Commands::DownloadModel { model_type } => {
            download_yolo_model(&model_type)?;
        }

        Commands::Demo { output } => {
            run_demo(&output)?;
        }
    }
    
    Ok(())
}

