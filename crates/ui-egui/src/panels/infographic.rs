//! Window › Infographic: CSV-driven infographic chart blocks, decorative SVG assets, and text shortcuts.
//! Every document mutation goes through `app.run("infographic.*", ...)` or existing engine commands.

use craft::ui::buttons::Button;
use craft::ui::contract::Show as _;
use craft::ui::display::{Divider, Shade, Text};
use egui::Ui;
use serde_json::{Value, json};

use crate::VectorcraftApp;

pub const ID: &str = "infographic";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Charts,
    Objects,
    Text,
}

#[derive(Clone)]
struct PanelState {
    tab: Tab,
    csv: String,
    category: String,
    values: String,
    title: String,
    subtitle: String,
    source_note: String,
    template_idx: usize,
    preset_idx: usize,
    highlight_idx: usize,
    tint: String,
    scale: f64,
    object_filter: String,
    cached_decorations: Vec<(String, String, String, bool)>,
    status: Option<String>,
    error: Option<String>,
    inspected_columns: Vec<String>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            tab: Tab::Charts,
            csv: "Month,Sales,Profit\nJan,120,35\nFeb,180,52\nMar,150,41\nApr,240,78\n".to_string(),
            category: "Month".to_string(),
            values: "Sales".to_string(),
            title: "Quarterly Performance".to_string(),
            subtitle: "Generated from CSV".to_string(),
            source_note: "Source: Internal data".to_string(),
            template_idx: 0,
            preset_idx: 0,
            highlight_idx: 2,
            tint: String::new(),
            scale: 1.0,
            object_filter: String::new(),
            cached_decorations: Vec::new(),
            status: None,
            error: None,
            inspected_columns: Vec::new(),
        }
    }
}

const TEMPLATES: &[(&str, &str)] = &[
    ("bar-report-basic", "Bar Report Basic (1920x1080)"),
    ("kpi-highlight-card", "KPI Highlight Card (1200x800)"),
    ("presentation-wide", "Presentation Wide (1600x900)"),
];

const PRESETS: &[(&str, &str)] = &[
    ("bar-building", "Building Bars"),
    ("bar-arrow-highlight", "Arrow Highlight Bars"),
    ("bar-isometric-blue", "Isometric Blue Growth"),
    ("bar-stepped-ribbon", "Stepped Ribbon Bars"),
    ("bar-decline-green-blue", "Green-Blue Contrast"),
    ("bar-isometric-orange", "Isometric Orange Rising"),
];

const HIGHLIGHTS: &[&str] = &["none", "last", "max", "min"];

pub fn show(app: &mut VectorcraftApp, ui: &mut Ui) {
    let state_id = ui.make_persistent_id("infographic_panel_state");
    let mut st: PanelState = ui.data_mut(|d| d.get_temp_mut_or_default::<PanelState>(state_id).clone());

    ui.horizontal(|ui| {
        if ui.selectable_label(st.tab == Tab::Charts, tl!("Charts")).clicked() {
            st.tab = Tab::Charts;
        }
        if ui.selectable_label(st.tab == Tab::Objects, tl!("Objects")).clicked() {
            st.tab = Tab::Objects;
        }
        if ui.selectable_label(st.tab == Tab::Text, tl!("Text")).clicked() {
            st.tab = Tab::Text;
        }
    });
    Divider::horizontal().show(ui);

    if let Some(err) = st.error.clone() {
        ui.colored_label(egui::Color32::from_rgb(220, 68, 68), err);
    } else if let Some(ok) = st.status.clone() {
        Text::new(ok).shade(Shade::Muted).show(ui);
    }

    egui::ScrollArea::vertical().show(ui, |ui| match st.tab {
        Tab::Charts => show_charts_tab(app, ui, &mut st),
        Tab::Objects => show_objects_tab(app, ui, &mut st),
        Tab::Text => show_text_tab(app, ui, &mut st),
    });

    ui.data_mut(|d| d.insert_temp(state_id, st));
}

fn show_charts_tab(app: &mut VectorcraftApp, ui: &mut Ui, st: &mut PanelState) {
    Text::new(tl!("CSV Data")).show(ui);
    ui.add(egui::TextEdit::multiline(&mut st.csv).desired_rows(4).desired_width(f32::INFINITY));

    if Button::new(tl!("Inspect CSV")).show(ui).clicked() {
        match app.run("infographic.inspectCsv", json!({ "csv": st.csv })) {
            Ok(v) => {
                st.error = None;
                st.inspected_columns = v
                    .get("columns")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().filter_map(|c| c.get("name").and_then(Value::as_str).map(str::to_owned)).collect())
                    .unwrap_or_default();
                st.status = Some(format!("Columns: {}", st.inspected_columns.join(", ")));
            }
            Err(e) => st.error = Some(e),
        }
    }

    ui.horizontal(|ui| {
        ui.label(tl!("Category"));
        ui.text_edit_singleline(&mut st.category);
    });
    ui.horizontal(|ui| {
        ui.label(tl!("Values"));
        ui.text_edit_singleline(&mut st.values);
    });

    egui::ComboBox::from_label(tl!("Template")).selected_text(TEMPLATES[st.template_idx].1).show_ui(ui, |ui| {
        for (i, (_, label)) in TEMPLATES.iter().enumerate() {
            ui.selectable_value(&mut st.template_idx, i, *label);
        }
    });

    egui::ComboBox::from_label(tl!("Preset")).selected_text(PRESETS[st.preset_idx].1).show_ui(ui, |ui| {
        for (i, (_, label)) in PRESETS.iter().enumerate() {
            ui.selectable_value(&mut st.preset_idx, i, *label);
        }
    });

    if PRESETS[st.preset_idx].0 == "bar-arrow-highlight" {
        egui::ComboBox::from_label(tl!("Highlight")).selected_text(HIGHLIGHTS[st.highlight_idx]).show_ui(ui, |ui| {
            for (i, mode) in HIGHLIGHTS.iter().enumerate() {
                ui.selectable_value(&mut st.highlight_idx, i, *mode);
            }
        });
    }

    ui.horizontal(|ui| {
        ui.label(tl!("Title"));
        ui.text_edit_singleline(&mut st.title);
    });
    ui.horizontal(|ui| {
        ui.label(tl!("Subtitle"));
        ui.text_edit_singleline(&mut st.subtitle);
    });
    ui.horizontal(|ui| {
        ui.label(tl!("Source"));
        ui.text_edit_singleline(&mut st.source_note);
    });

    let values_vec: Vec<String> = st.values.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mapping = json!({ "category": st.category.trim(), "values": values_vec });
    let preset = PRESETS[st.preset_idx].0;
    let highlight = json!({ "mode": HIGHLIGHTS[st.highlight_idx] });

    ui.horizontal(|ui| {
        if Button::new(tl!("Create Infographic")).show(ui).clicked() {
            let params = json!({
                "template": TEMPLATES[st.template_idx].0,
                "newDocument": true,
                "csv": st.csv,
                "mapping": mapping,
                "preset": preset,
                "highlight": highlight,
                "title": st.title,
                "subtitle": st.subtitle,
                "sourceNote": st.source_note,
            });
            match app.run("infographic.createFromCsv", params) {
                Ok(_) => {
                    st.error = None;
                    st.status = Some("Created infographic from CSV".into());
                }
                Err(e) => st.error = Some(e),
            }
        }

        if Button::new(tl!("Add Chart")).show(ui).clicked() {
            if app.session.doc().is_err() {
                if let Err(e) = app.run("file.new", json!({ "width": 1400, "height": 900 })) {
                    st.error = Some(e);
                    return;
                }
            }
            let existing_count = read_charts_from_app(app).len();
            let col = (existing_count % 2) as f64;
            let row = (existing_count / 2) as f64;
            let params = json!({
                "csv": st.csv,
                "mapping": mapping,
                "preset": preset,
                "highlight": highlight,
                "placement": {
                    "x": 80.0 + col * 680.0,
                    "y": 80.0 + row * 400.0,
                    "width": 620.0,
                    "height": 340.0
                }
            });
            match app.run("infographic.addChartFromCsv", params) {
                Ok(res) => {
                    st.error = None;
                    st.status = Some("Added chart block".into());
                    if let Some(gid) = res.get("graphNodeId").and_then(Value::as_u64) {
                        let _ = app.run("select.set", json!({ "ids": [gid] }));
                    }
                }
                Err(e) => st.error = Some(e),
            }
        }
    });

    Divider::horizontal().show(ui);
    Text::new(tl!("Active Charts")).show(ui);

    let charts = read_charts_from_app(app);
    if charts.is_empty() {
        Text::new(tl!("No infographic charts in document.")).shade(Shade::Muted).show(ui);
        return;
    }

    for (cid, gid, cpreset) in charts {
        ui.horizontal(|ui| {
            ui.label(format!("{cid} ({cpreset})"));
            if Button::new(tl!("Select")).show(ui).clicked() {
                if let Some(id) = gid {
                    let _ = app.run("select.set", json!({ "ids": [id] }));
                }
            }
            if Button::new(tl!("Update")).show(ui).clicked() {
                let params = json!({
                    "chartId": cid,
                    "csv": st.csv,
                    "mapping": mapping,
                    "preset": preset,
                    "highlight": highlight,
                });
                match app.run("infographic.updateChart", params) {
                    Ok(_) => {
                        st.error = None;
                        st.status = Some(format!("Updated {cid}"));
                    }
                    Err(e) => st.error = Some(e),
                }
            }
            if Button::new(tl!("Remove")).show(ui).clicked() {
                match app.run("infographic.removeChart", json!({ "chartId": cid })) {
                    Ok(_) => {
                        st.error = None;
                        st.status = Some(format!("Removed {cid}"));
                    }
                    Err(e) => st.error = Some(e),
                }
            }
        });
    }
}

fn show_objects_tab(app: &mut VectorcraftApp, ui: &mut Ui, st: &mut PanelState) {
    if st.cached_decorations.is_empty() {
        if let Ok(v) = app.run("infographic.listDecorations", json!({})) {
            if let Some(arr) = v.get("decorations").and_then(Value::as_array) {
                st.cached_decorations = arr
                    .iter()
                    .filter_map(|item| {
                        let id = item.get("id").and_then(Value::as_str)?.to_string();
                        let label = item.get("label").and_then(Value::as_str).unwrap_or(&id).to_string();
                        let cat = item.get("category").and_then(Value::as_str).unwrap_or("general").to_string();
                        let recolorable = item.get("recolorable").and_then(Value::as_bool).unwrap_or(false);
                        Some((id, label, cat, recolorable))
                    })
                    .collect();
            }
        }
    }

    Text::new(tl!("Decorative SVG Assets")).show(ui);
    ui.horizontal(|ui| {
        ui.label(tl!("Filter"));
        ui.text_edit_singleline(&mut st.object_filter);
    });
    ui.horizontal(|ui| {
        ui.label(tl!("Scale"));
        ui.add(egui::DragValue::new(&mut st.scale).range(0.25..=8.0).speed(0.1));
        ui.label(tl!("Tint"));
        ui.text_edit_singleline(&mut st.tint);
    });
    Divider::horizontal().show(ui);

    let q = st.object_filter.trim().to_ascii_lowercase();
    for (id, label, cat, recolorable) in &st.cached_decorations {
        if !q.is_empty()
            && !id.to_ascii_lowercase().contains(&q)
            && !label.to_ascii_lowercase().contains(&q)
            && !cat.to_ascii_lowercase().contains(&q)
        {
            continue;
        }
        ui.horizontal(|ui| {
            let tag = if *recolorable { cat.as_str() } else { "multi-color" };
            ui.label(format!("{label} [{tag}]"));
            if Button::new(tl!("Place")).show(ui).clicked() {
                let mut params = json!({
                    "asset": id,
                    "scale": st.scale,
                });
                if !st.tint.trim().is_empty() {
                    params["tint"] = json!(st.tint.trim());
                }
                match app.run("infographic.placeDecoration", params) {
                    Ok(_) => {
                        st.error = None;
                        st.status = Some(format!("Placed {id}"));
                    }
                    Err(e) => st.error = Some(e),
                }
            }
        });
    }
}

fn show_text_tab(app: &mut VectorcraftApp, ui: &mut Ui, st: &mut PanelState) {
    Text::new(tl!("Infographic Text Presets")).show(ui);
    ui.horizontal(|ui| {
        if Button::new(tl!("Add Title")).show(ui).clicked() {
            let text = if st.title.trim().is_empty() { "Infographic Title" } else { st.title.trim() };
            match app.run("text.create", json!({ "x": 80, "y": 64, "text": text, "size": 40 })) {
                Ok(_) => st.error = None,
                Err(e) => st.error = Some(e),
            }
        }
        if Button::new(tl!("Add Subtitle")).show(ui).clicked() {
            let text = if st.subtitle.trim().is_empty() { "Subtitle" } else { st.subtitle.trim() };
            match app.run("text.create", json!({ "x": 80, "y": 124, "text": text, "size": 20 })) {
                Ok(_) => st.error = None,
                Err(e) => st.error = Some(e),
            }
        }
        if Button::new(tl!("Add Note")).show(ui).clicked() {
            let text = if st.source_note.trim().is_empty() { "Source: Data" } else { st.source_note.trim() };
            match app.run("text.create", json!({ "x": 80, "y": 720, "text": text, "size": 14 })) {
                Ok(_) => st.error = None,
                Err(e) => st.error = Some(e),
            }
        }
    });

    Divider::horizontal().show(ui);
    Text::new(tl!("Existing Text Panels")).show(ui);
    ui.horizontal_wrapped(|ui| {
        for (label, panel_id) in
            [("Character", "character"), ("Paragraph", "paragraph"), ("Character Styles", "charStyles"), ("Paragraph Styles", "paraStyles")]
        {
            if Button::new(tl!(label)).show(ui).clicked() {
                let _ = app.run("window.panel", json!({ "panel": panel_id }));
            }
        }
    });
}

fn read_charts_from_app(app: &VectorcraftApp) -> Vec<(String, Option<u64>, String)> {
    let Ok(st) = app.session.doc() else {
        return Vec::new();
    };
    let doc = &st.doc;
    let Some(info_root) = doc.unknown.get("infographic") else {
        return Vec::new();
    };
    let Some(infos) = info_root.get("infographics").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for info in infos {
        if let Some(charts) = info.get("charts").and_then(Value::as_array) {
            for ch in charts {
                let cid = ch.get("id").and_then(Value::as_str).unwrap_or("chart").to_string();
                let gid = ch.get("graphNodeId").and_then(Value::as_u64);
                let preset = ch.get("preset").and_then(Value::as_str).unwrap_or("bar-building").to_string();
                out.push((cid, gid, preset));
            }
        }
    }
    out
}
