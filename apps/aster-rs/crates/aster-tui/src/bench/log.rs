use std::collections::BTreeMap;

use arbor_tui::testing::HeadlessFrameStats;
use aster_domain::ConversationStatus;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BenchProfile {
    HeadlessStrict,
    Interactive,
}

impl BenchProfile {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "headless" | "headless-strict" | "headless_strict" => Some(Self::HeadlessStrict),
            "interactive" => Some(Self::Interactive),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::HeadlessStrict => "headless_strict",
            Self::Interactive => "interactive",
        }
    }

    fn thresholds(self) -> WatermarkThresholds {
        match self {
            Self::HeadlessStrict => WatermarkThresholds {
                s_lt_ms: 0.5,
                a_lt_ms: 1.0,
                b_lt_ms: 2.0,
            },
            Self::Interactive => WatermarkThresholds {
                s_lt_ms: 4.0,
                a_lt_ms: 8.0,
                b_lt_ms: 16.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WatermarkThresholds {
    s_lt_ms: f32,
    a_lt_ms: f32,
    b_lt_ms: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) enum Watermark {
    S,
    A,
    B,
    C,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SceneTag {
    Idle,
    Streaming,
    Scrolling,
    PaletteOpen,
    ModelSwitch,
    Exit,
}

impl SceneTag {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Streaming => "streaming",
            Self::Scrolling => "scrolling",
            Self::PaletteOpen => "palette_open",
            Self::ModelSwitch => "model_switch",
            Self::Exit => "exit",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct StageTiming {
    pub(crate) events: f32,
    pub(crate) update: f32,
    pub(crate) pre_render: f32,
    pub(crate) render: f32,
    pub(crate) flush: f32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct FrameMeta {
    pub(crate) msg_lines: usize,
    pub(crate) signal_dirty: usize,
    pub(crate) dirty_render: usize,
    pub(crate) dirty_layout: usize,
    pub(crate) dirty_structure: usize,
    pub(crate) dirty_theme: usize,
    pub(crate) dirty_full: usize,
    pub(crate) dirty_regions: usize,
    pub(crate) new_tokens: usize,
    pub(crate) messages: usize,
    pub(crate) layout_cache_hits: usize,
    pub(crate) layout_cache_misses: usize,
    pub(crate) layout_cache_mismatches: usize,
    pub(crate) render_cache_hits: usize,
    pub(crate) render_cache_misses: usize,
    pub(crate) render_cache_mismatches: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct FrameLog {
    pub(crate) seq: u32,
    pub(crate) scene: SceneTag,
    pub(crate) total_ms: f32,
    pub(crate) stage: StageTiming,
    pub(crate) meta: FrameMeta,
    pub(crate) watermark: Watermark,
    pub(crate) bottleneck: &'static str,
}

impl FrameLog {
    pub(crate) fn from_stats(
        seq: u32,
        scene: SceneTag,
        stats: &HeadlessFrameStats,
        meta: FrameMeta,
        profile: BenchProfile,
    ) -> Self {
        let stage = StageTiming {
            events: us_to_ms(stats.events_us),
            update: us_to_ms(stats.update_us),
            pre_render: us_to_ms(stats.pre_render_us),
            render: us_to_ms(stats.render_us),
            flush: us_to_ms(stats.flush_us),
        };
        let total_ms = us_to_ms(stats.total_us);
        Self {
            seq,
            scene,
            total_ms,
            bottleneck: bottleneck(&stage),
            watermark: compute_watermark(total_ms, profile),
            stage,
            meta,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BenchSummary {
    frames: Vec<FrameLog>,
    profile: BenchProfile,
    budget: BenchBudget,
}

impl BenchSummary {
    pub(crate) fn new(frames: Vec<FrameLog>, profile: BenchProfile, budget: BenchBudget) -> Self {
        Self {
            frames,
            profile,
            budget,
        }
    }

    pub(crate) fn report(&self) -> String {
        let metrics = self.metrics();
        let total = metrics.total_frames;
        let avg = metrics.avg_ms;
        let p95 = metrics.p95_ms;
        let p99 = metrics.p99_ms;
        let counts = metrics.watermark_counts;
        let verdict = self.verdict();
        let budget_status = if verdict.passed { "PASS" } else { "FAIL" };
        let top_bottleneck = self.top_bottleneck().unwrap_or("n/a");
        let slowest_scene = self
            .slowest_scene()
            .map(|(scene, value)| format!("{} (avg {:.2}ms)", scene.as_str(), value))
            .unwrap_or_else(|| "n/a".to_string());
        let pct = |count: usize| {
            if total == 0 {
                0.0
            } else {
                (count as f32 / total as f32) * 100.0
            }
        };

        format!(
            "=== Bench Summary ===\n\
Profile: {}\n\
Total frames: {total}\n\
Avg frame ms: {avg:.3}\n\
P95 frame ms: {p95:.3}\n\
P99 frame ms: {p99:.3}\n\
S frames: {} ({:.1}%)\n\
A frames: {} ({:.1}%)\n\
B frames: {} ({:.1}%)\n\
C frames: {} ({:.1}%)\n\
Top bottleneck: {top_bottleneck}\n\
Slowest scene: {slowest_scene}\n\
Budget: {budget_status} (p95 <= {:.3}ms, p99 <= {:.3}ms, C frames <= {}%)\n",
            self.profile.name(),
            counts[0],
            pct(counts[0]),
            counts[1],
            pct(counts[1]),
            counts[2],
            pct(counts[2]),
            counts[3],
            pct(counts[3]),
            self.budget.max_p95_ms,
            self.budget.max_p99_ms,
            self.budget.max_c_frame_pct,
        )
    }

    pub(crate) fn verdict(&self) -> BenchVerdict {
        let metrics = self.metrics();
        let c_pct = if metrics.total_frames == 0 {
            0
        } else {
            metrics.watermark_counts[3] * 100 / metrics.total_frames
        };
        let mut failures = Vec::new();
        if metrics.p95_ms > self.budget.max_p95_ms {
            failures.push(format!(
                "p95 {:.3}ms exceeded {:.3}ms",
                metrics.p95_ms, self.budget.max_p95_ms
            ));
        }
        if metrics.p99_ms > self.budget.max_p99_ms {
            failures.push(format!(
                "p99 {:.3}ms exceeded {:.3}ms",
                metrics.p99_ms, self.budget.max_p99_ms
            ));
        }
        if c_pct > self.budget.max_c_frame_pct {
            failures.push(format!(
                "C frame pct {c_pct}% exceeded {}%",
                self.budget.max_c_frame_pct
            ));
        }

        BenchVerdict {
            passed: failures.is_empty(),
            failures,
        }
    }

    fn metrics(&self) -> BenchMetrics {
        BenchMetrics {
            total_frames: self.frames.len(),
            avg_ms: average(self.frames.iter().map(|frame| frame.total_ms)),
            p95_ms: percentile(self.frames.iter().map(|frame| frame.total_ms), 0.95),
            p99_ms: percentile(self.frames.iter().map(|frame| frame.total_ms), 0.99),
            watermark_counts: self.watermark_counts(),
        }
    }

    fn watermark_counts(&self) -> [usize; 4] {
        let mut counts = [0; 4];
        for frame in &self.frames {
            let index = match frame.watermark {
                Watermark::S => 0,
                Watermark::A => 1,
                Watermark::B => 2,
                Watermark::C => 3,
            };
            counts[index] += 1;
        }
        counts
    }

    fn top_bottleneck(&self) -> Option<&'static str> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for frame in self
            .frames
            .iter()
            .filter(|frame| matches!(frame.watermark, Watermark::B | Watermark::C))
        {
            *counts.entry(frame.bottleneck).or_default() += 1;
        }

        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name)
    }

    fn slowest_scene(&self) -> Option<(SceneTag, f32)> {
        let mut scenes: BTreeMap<SceneTag, (usize, f32)> = BTreeMap::new();
        for frame in &self.frames {
            let entry = scenes.entry(frame.scene).or_default();
            entry.0 += 1;
            entry.1 += frame.total_ms;
        }

        scenes
            .into_iter()
            .map(|(scene, (count, total_ms))| (scene, total_ms / count as f32))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BenchBudget {
    pub(crate) max_p95_ms: f32,
    pub(crate) max_p99_ms: f32,
    pub(crate) max_c_frame_pct: usize,
}

impl BenchBudget {
    pub(crate) fn for_profile(profile: BenchProfile) -> Self {
        match profile {
            BenchProfile::HeadlessStrict => Self {
                max_p95_ms: 1.0,
                max_p99_ms: 2.0,
                max_c_frame_pct: 0,
            },
            BenchProfile::Interactive => Self {
                max_p95_ms: 16.0,
                max_p99_ms: 32.0,
                max_c_frame_pct: 1,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BenchVerdict {
    pub(crate) passed: bool,
    pub(crate) failures: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BenchMetrics {
    total_frames: usize,
    avg_ms: f32,
    p95_ms: f32,
    p99_ms: f32,
    watermark_counts: [usize; 4],
}

pub(crate) fn compute_watermark(total_ms: f32, profile: BenchProfile) -> Watermark {
    let thresholds = profile.thresholds();
    if total_ms < thresholds.s_lt_ms {
        Watermark::S
    } else if total_ms < thresholds.a_lt_ms {
        Watermark::A
    } else if total_ms < thresholds.b_lt_ms {
        Watermark::B
    } else {
        Watermark::C
    }
}

pub(crate) fn meta_from_state(
    msg_lines: usize,
    new_tokens: usize,
    status: &ConversationStatus,
    message_count: usize,
    stats: &HeadlessFrameStats,
) -> FrameMeta {
    let streaming_token = matches!(status, ConversationStatus::Streaming { .. }) as usize;
        FrameMeta {
            msg_lines,
            signal_dirty: stats
                .frame_stats
                .dirty_widgets
                .saturating_add(streaming_token),
            dirty_render: stats.frame_stats.dirty_render_widgets,
            dirty_layout: stats.frame_stats.dirty_layout_widgets,
            dirty_structure: stats.frame_stats.dirty_structure_widgets,
            dirty_theme: stats.frame_stats.dirty_theme_widgets,
            dirty_full: stats.frame_stats.dirty_full_widgets,
            dirty_regions: stats.frame_stats.dirty_regions,
        new_tokens,
        messages: message_count,
        layout_cache_hits: stats.frame_stats.layout_cache_hits,
        layout_cache_misses: stats.frame_stats.layout_cache_misses,
        layout_cache_mismatches: stats.frame_stats.layout_cache_mismatches,
        render_cache_hits: stats.frame_stats.render_cache_hits,
        render_cache_misses: stats.frame_stats.render_cache_misses,
        render_cache_mismatches: stats.frame_stats.render_cache_mismatches,
    }
}

fn bottleneck(stage: &StageTiming) -> &'static str {
    let stages = [
        ("events", stage.events),
        ("logic", stage.update + stage.pre_render),
        ("render", stage.render),
        ("io", stage.flush),
    ];
    let total = stages.iter().map(|(_, value)| *value).sum::<f32>();
    let Some((name, value)) = stages
        .into_iter()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
    else {
        return "none";
    };
    if total > 0.0 && value / total >= 0.5 {
        name
    } else {
        "mixed"
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn percentile(values: impl Iterator<Item = f32>, p: f32) -> f32 {
    let mut values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    let index = ((values.len().saturating_sub(1)) as f32 * p).ceil() as usize;
    values[index.min(values.len() - 1)]
}

fn us_to_ms(value: u64) -> f32 {
    value as f32 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_thresholds_are_stable() {
        assert_eq!(
            compute_watermark(0.49, BenchProfile::HeadlessStrict),
            Watermark::S
        );
        assert_eq!(
            compute_watermark(0.5, BenchProfile::HeadlessStrict),
            Watermark::A
        );
        assert_eq!(
            compute_watermark(1.0, BenchProfile::HeadlessStrict),
            Watermark::B
        );
        assert_eq!(
            compute_watermark(2.0, BenchProfile::HeadlessStrict),
            Watermark::C
        );
        assert_eq!(
            compute_watermark(3.99, BenchProfile::Interactive),
            Watermark::S
        );
        assert_eq!(
            compute_watermark(4.0, BenchProfile::Interactive),
            Watermark::A
        );
    }

    #[test]
    fn summary_reports_core_waterlines() {
        let frames = vec![
            frame(0, SceneTag::Idle, 1.0, Watermark::S, "render"),
            frame(1, SceneTag::Streaming, 9.0, Watermark::B, "render"),
            frame(2, SceneTag::Streaming, 20.0, Watermark::C, "logic"),
        ];

        let report = BenchSummary::new(
            frames,
            BenchProfile::HeadlessStrict,
            BenchBudget::for_profile(BenchProfile::HeadlessStrict),
        )
        .report();

        assert!(report.contains("Total frames: 3"));
        assert!(report.contains("Profile: headless_strict"));
        assert!(report.contains("S frames: 1"));
        assert!(report.contains("Slowest scene: streaming"));
        assert!(report.contains("Budget: FAIL"));
    }

    #[test]
    fn verdict_passes_and_fails_budget() {
        let budget = BenchBudget {
            max_p95_ms: 1.0,
            max_p99_ms: 2.0,
            max_c_frame_pct: 0,
        };
        let passing = BenchSummary::new(
            vec![frame(0, SceneTag::Idle, 0.4, Watermark::S, "render")],
            BenchProfile::HeadlessStrict,
            budget,
        );
        assert!(passing.verdict().passed);

        let failing = BenchSummary::new(
            vec![
                frame(0, SceneTag::Idle, 0.4, Watermark::S, "render"),
                frame(1, SceneTag::Idle, 2.5, Watermark::C, "render"),
            ],
            BenchProfile::HeadlessStrict,
            budget,
        );
        let verdict = failing.verdict();
        assert!(!verdict.passed);
        assert!(verdict
            .failures
            .iter()
            .any(|failure| failure.contains("p99")));
    }

    fn frame(
        seq: u32,
        scene: SceneTag,
        total_ms: f32,
        watermark: Watermark,
        bottleneck: &'static str,
    ) -> FrameLog {
        FrameLog {
            seq,
            scene,
            total_ms,
            stage: StageTiming::default(),
            meta: FrameMeta::default(),
            watermark,
            bottleneck,
        }
    }
}
