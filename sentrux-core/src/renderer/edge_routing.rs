//! Dashed polyline drawing — renders edges with continuous dash/gap patterns.
//!
//! The dash pattern is maintained across polyline bends (no resets at corners).
//! Animation is achieved by shifting the phase offset each frame.

use egui::{Pos2, Stroke};

/// Draw a dashed polyline along `pts` with the given dash/gap pattern.
/// The pattern is continuous across bends (no resets at corners).
/// `offset` animates the pattern by shifting the phase.
pub(crate) fn draw_dashed_polyline(
    painter: &egui::Painter,
    pts: &[Pos2],
    stroke: Stroke,
    offset: f32,
    dash_len: f32,
    gap_len: f32,
) {
    let cycle = dash_len + gap_len;
    // Guard: zero cycle OR zero gap = solid line — draw segments directly.
    // Previously gap_len=0 caused 2x iterations (every gap phase was a no-op
    // where t didn't advance). Import edges (solid) wasted half their iterations.
    if cycle < f32::EPSILON || gap_len < f32::EPSILON {
        for i in 0..pts.len().saturating_sub(1) {
            painter.line_segment([pts[i], pts[i + 1]], stroke);
        }
        return;
    }

    // First pass: compute total polyline length and per-segment cumulative starts.
    // Second pass: walk in dash/gap strides using cumulative distance so the
    // pattern is continuous across bend points. Previously each segment reset
    // the pattern, causing visible jumps at L-path bends for call/inherit edges.
    let n = pts.len().saturating_sub(1);
    if n == 0 {
        return;
    }
    // Use a stack buffer for polylines with <= 8 segments (typical L-paths have 3).
    // Falls back to heap allocation for rare complex polylines.
    const STACK_CAP: usize = 8;
    let mut stack_buf = [0.0_f32; STACK_CAP];
    let mut heap_buf = Vec::new();
    let seg_starts: &mut [f32] = if n <= STACK_CAP {
        &mut stack_buf[..n]
    } else {
        heap_buf.resize(n, 0.0_f32);
        &mut heap_buf[..]
    };
    let mut cum = 0.0_f32;
    for i in 0..n {
        seg_starts[i] = cum;
        let dx = pts[i + 1].x - pts[i].x;
        let dy = pts[i + 1].y - pts[i].y;
        cum += (dx * dx + dy * dy).sqrt();
    }
    let total_len = cum;
    if total_len < 0.5 {
        return;
    }

    // Walk the polyline in dash/gap strides using global distance.
    // `d` = current position along the entire polyline (0..total_len).
    // Animation offset shifts the pattern backward so dashes march forward.
    let phase = offset % cycle;
    let mut d = phase - cycle;
    let mut drawing = true;

    while d < total_len {
        let stride = if drawing { dash_len } else { gap_len };
        let d_end = (d + stride).min(total_len);

        if drawing {
            // Emit visible portion: clamp to [0, total_len].
            let vis_start = d.max(0.0);
            let vis_end = d_end;
            if vis_end > vis_start + 0.01 {
                emit_dash_segments(painter, pts, seg_starts, total_len, vis_start, vis_end, stroke);
            }
        }

        d = d_end;
        drawing = !drawing;
    }
}

/// Emit line segments for a dash that spans [vis_start, vis_end] along the polyline.
/// When a dash stride crosses one or more segment boundaries, emits one line
/// segment per crossed polyline segment so the line follows the bend.
fn emit_dash_segments(
    painter: &egui::Painter,
    pts: &[Pos2],
    seg_starts: &[f32],
    total_len: f32,
    vis_start: f32,
    vis_end: f32,
    stroke: Stroke,
) {
    // Find which polyline segments are spanned by [vis_start, vis_end].
    let seg_first = segment_at(seg_starts, vis_start);
    let seg_last = segment_at(seg_starts, vis_end);

    if seg_first == seg_last {
        // Common fast path: dash fits within one segment.
        let p0 = point_at_distance(pts, seg_starts, vis_start);
        let p1 = point_at_distance(pts, seg_starts, vis_end);
        painter.line_segment([p0, p1], stroke);
    } else {
        // Dash crosses bend point(s) — emit sub-segments.
        let mut prev_pt = point_at_distance(pts, seg_starts, vis_start);
        for seg in seg_first..seg_last {
            // End of this segment = start of the next one.
            let seg_end_d = if seg + 1 < seg_starts.len() {
                seg_starts[seg + 1]
            } else {
                total_len
            };
            let next_pt = point_at_distance(pts, seg_starts, seg_end_d);
            painter.line_segment([prev_pt, next_pt], stroke);
            prev_pt = next_pt;
        }
        // Final sub-segment to the dash end.
        let end_pt = point_at_distance(pts, seg_starts, vis_end);
        painter.line_segment([prev_pt, end_pt], stroke);
    }
}

/// Find which polyline segment contains the given cumulative distance.
#[inline]
fn segment_at(seg_starts: &[f32], dist: f32) -> usize {
    let n = seg_starts.len();
    match seg_starts.binary_search_by(|s| {
        s.partial_cmp(&dist).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Ok(i) => i.min(n - 1),
        Err(i) => if i > 0 { i - 1 } else { 0 },
    }
}

/// Interpolate a point along a polyline at a given cumulative distance.
/// `seg_starts[i]` = cumulative distance at the start of segment i.
#[inline]
fn point_at_distance(pts: &[Pos2], seg_starts: &[f32], dist: f32) -> Pos2 {
    let n = seg_starts.len();
    // Find which segment contains this distance via binary search
    let seg = match seg_starts.binary_search_by(|s| {
        s.partial_cmp(&dist).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Ok(i) => i.min(n - 1),
        Err(i) => if i > 0 { i - 1 } else { 0 },
    };
    let seg_start_d = seg_starts[seg];
    let dx = pts[seg + 1].x - pts[seg].x;
    let dy = pts[seg + 1].y - pts[seg].y;
    let seg_len = (dx * dx + dy * dy).sqrt();
    let t = if seg_len > 0.001 {
        (dist - seg_start_d) / seg_len
    } else {
        0.0
    };
    let t = t.clamp(0.0, 1.0);
    egui::pos2(pts[seg].x + dx * t, pts[seg].y + dy * t)
}
