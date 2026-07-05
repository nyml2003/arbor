use std::time::Instant;

use arbor_tui_application::app::{FrameStats, RenderResult};

pub struct FrameAccumulator {
    rendered_frames: u64,
    throttled_frames: u64,
    idle_frames: u64,
    total_layout_us: u64,
    total_render_us: u64,
    total_diff_us: u64,
    total_emit_us: u64,
    total_emit_queue_us: u64,
    total_emit_flush_us: u64,
    total_total_us: u64,
    min_frame_us: u64,
    max_frame_us: u64,
    last_frame_us: u64,
    start_time: Instant,
}

impl FrameAccumulator {
    pub fn new() -> Self {
        Self {
            rendered_frames: 0,
            throttled_frames: 0,
            idle_frames: 0,
            total_layout_us: 0,
            total_render_us: 0,
            total_diff_us: 0,
            total_emit_us: 0,
            total_emit_queue_us: 0,
            total_emit_flush_us: 0,
            total_total_us: 0,
            min_frame_us: u64::MAX,
            max_frame_us: 0,
            last_frame_us: 0,
            start_time: Instant::now(),
        }
    }

    pub fn record(&mut self, stats: &FrameStats, result: RenderResult) {
        match result {
            RenderResult::Rendered => {
                self.rendered_frames += 1;
                self.total_layout_us += stats.layout_us;
                self.total_render_us += stats.render_us;
                self.total_diff_us += stats.diff_us;
                self.total_emit_us += stats.emit_us;
                self.total_emit_queue_us += stats.emit_queue_us;
                self.total_emit_flush_us += stats.emit_flush_us;
                self.total_total_us += stats.total_us;
                self.min_frame_us = self.min_frame_us.min(stats.total_us);
                self.max_frame_us = self.max_frame_us.max(stats.total_us);
                self.last_frame_us = stats.total_us;
            }
            RenderResult::Throttled => self.throttled_frames += 1,
            RenderResult::NothingChanged => self.idle_frames += 1,
        }
    }

    pub fn fps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.rendered_frames as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn last_frame_us(&self) -> u64 {
        self.last_frame_us
    }

    pub fn report(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rendered = self.rendered_frames.max(1);
        let avg = |value: u64| value as f64 / rendered as f64;
        let min_frame = if self.rendered_frames > 0 {
            self.min_frame_us
        } else {
            0
        };

        format!(
            "\n{sep}\n Aster - Performance Report\n{sep}\n\
            Elapsed:{elapsed:>17.3} s\n  Frames rendered:{rendered_frames:>10}\n\
            Throttled (16ms):{throttled:>10}\n  Idle (no change):{idle:>10}\n\
            Avg FPS:{fps:>17.1}\n\n  Per-frame timing (n={rendered}):\n\
            Layout:      avg {layout}\n  Render:      avg {render}\n\
            Diff:        avg {diff}\n  Emit queue:  avg {emit_queue}\n\
            Emit flush:  avg {emit_flush}\n  Emit total:  avg {emit}\n\
            -----------------------------\n\
            Total:       avg {total}   min {min}   max {max}\n{sep}\n",
            sep = "===========================================",
            rendered_frames = self.rendered_frames,
            throttled = self.throttled_frames,
            idle = self.idle_frames,
            fps = self.fps(),
            layout = format_duration(avg(self.total_layout_us) as u64),
            render = format_duration(avg(self.total_render_us) as u64),
            diff = format_duration(avg(self.total_diff_us) as u64),
            emit_queue = format_duration(avg(self.total_emit_queue_us) as u64),
            emit_flush = format_duration(avg(self.total_emit_flush_us) as u64),
            emit = format_duration(avg(self.total_emit_us) as u64),
            total = format_duration(avg(self.total_total_us) as u64),
            min = format_duration(min_frame),
            max = format_duration(self.max_frame_us),
        )
    }
}

fn format_duration(us: u64) -> String {
    if us < 1000 {
        format!("{us:>4} us")
    } else if us < 1_000_000 {
        format!("{:>5.1} ms", us as f64 / 1000.0)
    } else {
        format!("{:>5.2} s", us as f64 / 1_000_000.0)
    }
}
