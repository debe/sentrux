//! Bottom status bar — contextual file info, zoom level, and health badges.
//!
//! Shows different content depending on interaction state:
//! idle (root path), hover (file details), selected (edge counts).

use super::state::AppState;

/// Draw the bottom status bar.
/// Idle: show abs path of root. Hover: show hovered file info. Selected: show selected + edges.
pub fn draw_status_bar(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal(|ui| {
        draw_left_info(ui, state);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            draw_right_stats(ui, state);
        });
    });
}

/// Left side: file path, hover details, or idle message.
fn draw_left_info(ui: &mut egui::Ui, state: &AppState) {
    // Helper: prepend root to get absolute path
    let abs = |rel: &str| -> String {
        match &state.root_path {
            Some(root) => format!("{}/{}", root, rel),
            None => rel.to_string(),
        }
    };

    if let Some(path) = &state.hovered_path {
        // Hover: show file details [ref:56e27d59]
        if let Some(entry) = state.file_index.get(path) {
            ui.label(egui::RichText::new(abs(path)).strong().monospace());
            ui.label(egui::RichText::new(format!(
                "{}  {}  {} lines  {} logic  {} functions",
                entry.lang, entry.gs, entry.lines, entry.logic, entry.funcs
            )).weak().monospace());
        } else {
            ui.label(egui::RichText::new(abs(path)).monospace());
        }
    } else if let Some(path) = &state.selected_path {
        let (imports, calls, inherits) = edge_counts_for(state, path);
        let total = imports + calls + inherits;
        ui.label(egui::RichText::new(abs(path)).strong().monospace());
        ui.label(egui::RichText::new(format!(
            "{} edges  ({} import {} call {} inherit)",
            total, imports, calls, inherits
        )).weak().monospace());
    } else if let Some(root) = &state.root_path {
        // Idle: show absolute path of opened folder
        ui.label(egui::RichText::new(root.as_str()).weak().monospace());
    } else {
        ui.label(egui::RichText::new("Open a folder to begin").weak().monospace());
    }
}

/// Right side: zoom percentage, compact grades, file/edge counts.
fn draw_right_stats(ui: &mut egui::Ui, state: &AppState) {
    let vp = &state.viewport;
    ui.label(egui::RichText::new(format!("{:.0}%", vp.scale * 100.0)).weak().monospace());

    // Compact grade display — just the overall letter, no dimension breakdown
    if let Some(arch) = &state.arch_report {
        let c = grade_color(arch.arch_grade);
        ui.label(egui::RichText::new(format!("Arch:{}", arch.arch_grade)).monospace().color(c));
    }
    if let Some(report) = &state.health_report {
        let c = grade_color(report.grade);
        ui.label(egui::RichText::new(format!("Health:{}", report.grade)).monospace().color(c));
    }

    draw_edge_file_counts(ui, state);
}

/// Edge and file/dir count display.
fn draw_edge_file_counts(ui: &mut egui::Ui, state: &AppState) {
    if let Some(snap) = &state.snapshot {
        ui.separator();
        let n_imp = snap.import_graph.len();
        let n_call = snap.call_graph.len();
        let n_inh = snap.inherit_graph.len();
        let total_edges = n_imp + n_call + n_inh;
        if total_edges > 0 {
            // ASCII-only edge stats — avoids fallback font for unicode symbols
            ui.label(egui::RichText::new(format!(
                "{} import  {} call  {} inherit",
                n_imp, n_call, n_inh
            )).weak().monospace());
            ui.separator();
        } else if state.scanning {
            ui.label(egui::RichText::new("edges ...").weak().monospace());
            ui.separator();
        }
        ui.label(egui::RichText::new(format!(
            "{} files  {} dirs",
            snap.total_files, snap.total_dirs
        )).weak().monospace());
    }
}

/// Map a letter grade to a color.
fn grade_color(grade: char) -> egui::Color32 {
    match grade {
        'A' => egui::Color32::from_rgb(100, 200, 100),
        'B' => egui::Color32::from_rgb(160, 200, 100),
        'C' => egui::Color32::from_rgb(200, 180, 80),
        'D' => egui::Color32::from_rgb(200, 120, 60),
        _ => egui::Color32::from_rgb(200, 80, 80),
    }
}

/// Count edges connected to a file from render_data (what's actually drawn).
/// Respects the active edge_filter so counts match what's visible on canvas. [ref:4e8f1175]
fn edge_counts_for(state: &AppState, path: &str) -> (usize, usize, usize) {
    let rd = match &state.render_data {
        Some(rd) => rd,
        None => return (0, 0, 0),
    };
    let mut imports = 0;
    let mut calls = 0;
    let mut inherits = 0;
    for ep in &rd.edge_paths {
        if !state.edge_filter.accepts(&ep.edge_type) {
            continue;
        }
        if ep.from_file == path || ep.to_file == path {
            match ep.edge_type.as_str() {
                "import" => imports += 1,
                "call" => calls += 1,
                "inherit" => inherits += 1,
                _ => {}
            }
        }
    }
    (imports, calls, inherits)
}
