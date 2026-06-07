use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use super::segment::Segment;

pub struct VideoExporter {
    output_dir: PathBuf,
}

impl VideoExporter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub fn export_segment(
        &self,
        input_path: &Path,
        segment: &Segment,
        output_path: &Path,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(output_path.parent().unwrap_or(&self.output_dir))?;

        let status = Command::new("ffmpeg")
            .arg("-ss")
            .arg(format!("{:.3}", segment.start))
            .arg("-i")
            .arg(input_path.to_str().unwrap())
            .arg("-to")
            .arg(format!("{:.3}", segment.end - segment.start))
            .arg("-c")
            .arg("copy")
            .arg("-avoid_negative_ts")
            .arg("make_zero")
            .arg("-reset_timestamps")
            .arg("1")
            .arg("-y")
            .arg(output_path.to_str().unwrap())
            .status()?;

        if !status.success() {
            anyhow::bail!("FFmpeg failed to export segment");
        }

        Ok(output_path.to_path_buf())
    }

    pub fn export_batch(
        &self,
        input_path: &Path,
        segments: &[Segment],
        prefix: &str,
    ) -> Result<Vec<PathBuf>> {
        let video_name = input_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        let video_output_dir = self.output_dir.join(video_name.as_ref());
        std::fs::create_dir_all(&video_output_dir)?;

        let mut output_paths = Vec::new();

        for (i, segment) in segments.iter().enumerate() {
            let filename = format!("{}_{:04}.mp4", prefix, i + 1);
            let output_path = video_output_dir.join(&filename);

            self.export_segment(input_path, segment, &output_path)?;

            println!("Segment {} → {}", i + 1, output_path.display());
            output_paths.push(output_path);
        }

        Ok(output_paths)
    }
}
