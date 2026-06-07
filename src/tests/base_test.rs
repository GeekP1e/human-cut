mod config;
mod detection;
mod processing;

use config::Config;
use detection::{Detector};
use detection::yolo::YoloDetector;
use processing::segment::{SegmentBatch};
use processing::merger::SegmentMerger;
use processing::exporter::VideoExporter;
use processing::reader::VideoReader;


#[cfg(test)]
mod tests {
    use super::*;
    use detection::Detection;

    fn detection(timestamp: f64, has_humans: bool) -> Detection {
        Detection {
            timestamp,
            has_humans,
            confidence: if has_humans { 0.8 } else { 0.0 },
            bboxes: Vec::new(),
        }
    }

    #[test]
    fn detect_segments_keeps_segments_above_min_duration() {
        let mut config = Config::default();
        config.min_segment_duration = 1.0;

        let detections = vec![
            detection(0.0, false),
            detection(1.0, true),
            detection(2.0, true),
            detection(2.5, false),
            detection(4.0, true),
            detection(4.2, false),
        ];

        let segments = detect_segments(&detections, &config);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start, 1.0);
        assert_eq!(segments[0].end, 2.0);
        assert_eq!(segments[0].confidence, 0.8);
        assert_eq!(segments[0].frame_count, 2);
    }

    #[test]
    fn segment_merger_merges_segments_with_small_gap() {
        let merger = SegmentMerger::new(0.5);
        let segments = vec![
            Segment::new(0.0, 1.0, 0.6),
            Segment::new(1.4, 2.0, 0.9),
            Segment::new(3.0, 4.0, 0.7),
        ];

        let merged = merger.merge(&segments);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start, 0.0);
        assert_eq!(merged[0].end, 2.0);
        assert_eq!(merged[0].confidence, 0.9);
    }
}