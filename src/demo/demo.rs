use std::path::{PathBuf, Path};
use anyhow::{Result};
use crate::processing::process::process_video;
use crate::config::Config;

pub fn run_demo(output_dir: &Path) -> Result<()> {
    let demo_path = PathBuf::from("./videos/demo_people.mp4");
    let demo_url = "https://github.com/intel-iot-devkit/sample-videos/raw/master/people-detection.mp4";

    if !demo_path.exists() {
        println!("📥 Downloading demo video...");
        std::fs::create_dir_all("./videos")?;

        let response = reqwest::blocking::get(demo_url)?;
        let bytes = response.bytes()?;

        if bytes.is_empty() {
            anyhow::bail!("Failed to download demo video");
        }

        std::io::Write::write_all(&mut std::fs::File::create(&demo_path)?, &bytes)?;
        println!("✅ Demo video saved to {:?}", demo_path);
    } else {
        println!("✅ Demo video already exists: {:?}", demo_path);
    }

    let mut cfg = Config::default();
    cfg.output_dir = output_dir.to_path_buf();
    cfg.yolo_confidence = 0.35;
    cfg.min_segment_duration = 2.0;
    cfg.merge_gap_seconds = 3.0;
    cfg.sample_rate_frames = 5;

    println!("\n🎬 Running demo...");
    process_video(&demo_path, &cfg)?;

    Ok(())
}