use super::segment::Segment;

pub struct SegmentMerger {
    gap_threshold: f64,
}

impl SegmentMerger {
    pub fn new(gap_threshold: f64) -> Self {
        Self { gap_threshold }
    }
    
    pub fn merge(&self, segments: &[Segment]) -> Vec<Segment> {
        if segments.is_empty() {
            return Vec::new();
        }
        
        let mut merged = Vec::new();
        let mut current = segments[0].clone();
        
        for segment in &segments[1..] {
            let gap = segment.start - current.end;
            
            if gap <= self.gap_threshold {
                current.end = segment.end;
                current.confidence = current.confidence.max(segment.confidence);
                current.frame_count += segment.frame_count;
            } else {
                merged.push(current);
                current = segment.clone();
            }
        }
        
        merged.push(current);
        merged
    }
    
    pub fn filter_by_duration(&self, segments: &[Segment], min_duration: f64) -> Vec<Segment> {
        segments.iter()
            .filter(|seg| seg.duration() >= min_duration)
            .cloned()
            .collect()
    }
}