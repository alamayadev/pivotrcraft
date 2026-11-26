use serde_json::{Value, json};

use super::chart::create_chart;
use super::csv::{Delimiter, decode_csv_bytes, load_csv_input, parse_rows};
use super::decorations::{find_decoration, place_decoration};
use super::presets::is_supported_preset;
use super::{bad_param, text_or};

const C_CREATE: &str = "infographic.createFromCsv";

#[derive(Clone, Copy)]
pub struct SlotRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy)]
pub struct TemplateDecoration {
    pub asset: &'static str,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy)]
pub struct TemplateSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub width: f64,
    pub height: f64,
    pub default_preset: &'static str,
    pub title: SlotRect,
    pub subtitle: SlotRect,
    pub chart: SlotRect,
    pub source: SlotRect,
    pub decorations: &'static [TemplateDecoration],
}

const TEMPLATES: &[TemplateSpec] = &[
    TemplateSpec {
        id: "bar-report-basic",
        label: "Bar Report Basic (16:9)",
        width: 1920.0,
        height: 1080.0,
        default_preset: "bar-building",
        title: SlotRect { x: 96.0, y: 72.0, width: 1360.0, height: 64.0 },
        subtitle: SlotRect { x: 96.0, y: 144.0, width: 1360.0, height: 36.0 },
        chart: SlotRect { x: 96.0, y: 220.0, width: 1728.0, height: 740.0 },
        source: SlotRect { x: 96.0, y: 1004.0, width: 1200.0, height: 28.0 },
        decorations: &[TemplateDecoration { asset: "accent-dots", x: 1720.0, y: 72.0, width: 96.0, height: 72.0 }],
    },
    TemplateSpec {
        id: "kpi-highlight-card",
        label: "KPI Highlight Card",
        width: 1200.0,
        height: 800.0,
        default_preset: "bar-arrow-highlight",
        title: SlotRect { x: 72.0, y: 56.0, width: 900.0, height: 56.0 },
        subtitle: SlotRect { x: 72.0, y: 118.0, width: 900.0, height: 32.0 },
        chart: SlotRect { x: 72.0, y: 184.0, width: 1056.0, height: 520.0 },
        source: SlotRect { x: 72.0, y: 736.0, width: 900.0, height: 24.0 },
        decorations: &[TemplateDecoration { asset: "trend-up", x: 1064.0, y: 56.0, width: 64.0, height: 64.0 }],
    },
    TemplateSpec {
        id: "presentation-wide",
        label: "Presentation Wide (1600x900)",
        width: 1600.0,
        height: 900.0,
        default_preset: "bar-building",
        title: SlotRect { x: 80.0, y: 64.0, width: 1120.0, height: 56.0 },
        subtitle: SlotRect { x: 80.0, y: 128.0, width: 1120.0, height: 32.0 },
        chart: SlotRect { x: 80.0, y: 196.0, width: 1440.0, height: 612.0 },
        source: SlotRect { x: 80.0, y: 836.0, width: 1000.0, height: 24.0 },
        decorations: &[TemplateDecoration { asset: "callout-ribbon", x: 1280.0, y: 64.0, width: 240.0, height: 48.0 }],
    },
];

pub fn find_template(id: &str) -> Option<&'static TemplateSpec> {
    TEMPLATES.iter().find(|t| t.id == id)
}

pub fn list_templates() -> Value {
    let items: Vec<Value> = TEMPLATES
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "label": t.label,
                "size": {"width": t.width, "height": t.height},
                "defaultPreset": t.default_preset,
                "slots": {
                    "title": slot_json(t.title),
                    "subtitle": slot_json(t.subtitle),
                    "chart": slot_json(t.chart),
                    "source": slot_json(t.source),
                },
                "decorations": t.decorations.iter().map(|d| json!({
                    "asset": d.asset,
                    "rect": [d.x, d.y, d.width, d.height],
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "templates": items })
}

fn slot_json(s: SlotRect) -> Value {
    json!({"x": s.x, "y": s.y, "width": s.width, "height": s.height})
}

pub fn create_from_csv(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let template_id = text_or(p, "template").unwrap_or("bar-report-basic");
    let tpl = find_template(template_id).ok_or_else(|| bad_param(C_CREATE, format!("unknown template `{template_id}`")))?;

    let preset = text_or(p, "preset").unwrap_or(tpl.default_preset);
    if !is_supported_preset(preset) {
        return Err(bad_param(C_CREATE, format!("unknown chart preset `{preset}`")));
    }

    // Validate CSV and mapping before any document mutation.
    let input = load_csv_input(p, C_CREATE)?;
    let text = decode_csv_bytes(&input.bytes, C_CREATE)?;
    let rows = parse_rows(&text, Delimiter::Auto);
    if rows.is_empty() {
        return Err(bad_param(C_CREATE, "CSV has no rows"));
    }
    let mapping = p.get("mapping").cloned().ok_or_else(|| bad_param(C_CREATE, "missing `mapping`"))?;
    let cat_col = mapping
        .get("category")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_param(C_CREATE, "mapping.category is required"))?;
    let header = &rows[0];
    if !header.iter().any(|h| h.trim() == cat_col) {
        return Err(bad_param(C_CREATE, format!("missing CSV column `{cat_col}`")));
    }
    let values = mapping.get("values").and_then(Value::as_array).ok_or_else(|| bad_param(C_CREATE, "mapping.values must be an array"))?;
    if values.is_empty() {
        return Err(bad_param(C_CREATE, "mapping.values must contain at least one column"));
    }
    for v in values {
        let Some(col) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(bad_param(C_CREATE, "mapping.values entries must be non-empty strings"));
        };
        if !header.iter().any(|h| h.trim() == col) {
            return Err(bad_param(C_CREATE, format!("missing CSV column `{col}`")));
        }
    }

    // Validate custom decorations if provided.
    let custom_decos = p.get("decorations").and_then(Value::as_array);
    if let Some(arr) = custom_decos {
        for item in arr {
            let asset = item.get("asset").and_then(Value::as_str).unwrap_or("");
            if find_decoration(asset).is_none() {
                return Err(bad_param(C_CREATE, format!("unknown decoration `{asset}`")));
            }
        }
    }

    // Validation complete. Mutate below.
    let new_doc = p.get("newDocument").and_then(Value::as_bool).unwrap_or(false) || s.doc().is_err();
    if new_doc {
        s.execute("file.new", &json!({"width": tpl.width, "height": tpl.height}))?;
    }

    let infographic_id = text_or(p, "infographicId").unwrap_or("infographic-1");
    let mut text_ids = Vec::new();
    if let Some(title) = text_or(p, "title") {
        let r = s.execute(
            "text.create",
            &json!({"x": tpl.title.x, "y": tpl.title.y, "text": title, "size": 40, "area": {"width": tpl.title.width, "height": tpl.title.height}}),
        )?;
        if let Some(id) = r.get("id").and_then(Value::as_u64) {
            text_ids.push(id);
        }
    }
    if let Some(subtitle) = text_or(p, "subtitle") {
        let r = s.execute(
            "text.create",
            &json!({"x": tpl.subtitle.x, "y": tpl.subtitle.y, "text": subtitle, "size": 20, "area": {"width": tpl.subtitle.width, "height": tpl.subtitle.height}}),
        )?;
        if let Some(id) = r.get("id").and_then(Value::as_u64) {
            text_ids.push(id);
        }
    }
    if let Some(source_note) = text_or(p, "sourceNote") {
        let r = s.execute(
            "text.create",
            &json!({"x": tpl.source.x, "y": tpl.source.y, "text": source_note, "size": 14, "area": {"width": tpl.source.width, "height": tpl.source.height}}),
        )?;
        if let Some(id) = r.get("id").and_then(Value::as_u64) {
            text_ids.push(id);
        }
    }

    let mut chart_params = json!({
        "infographicId": infographic_id,
        "csvData": text,
        "mapping": mapping,
        "preset": preset,
        "placement": {
            "x": tpl.chart.x,
            "y": tpl.chart.y,
            "width": tpl.chart.width,
            "height": tpl.chart.height,
        },
    });
    if let Some(axis) = p.get("axis") {
        chart_params["axis"] = axis.clone();
    }
    if let Some(hl) = p.get("highlight") {
        chart_params["highlight"] = hl.clone();
    } else if preset == "bar-arrow-highlight" {
        chart_params["highlight"] = json!({"mode": "max"});
    }
    if let Some(name) = text_or(p, "name") {
        chart_params["name"] = json!(name);
    }
    let chart_res = create_chart(s, &chart_params)?;

    let mut decoration_ids = Vec::new();
    let include_template_decos = p.get("includeTemplateDecorations").and_then(Value::as_bool).unwrap_or(true);
    if include_template_decos && custom_decos.is_none() {
        for d in tpl.decorations {
            let dr = place_decoration(
                s,
                &json!({
                    "infographicId": infographic_id,
                    "asset": d.asset,
                    "rect": [d.x, d.y, d.width, d.height],
                }),
            )?;
            if let Some(ids) = dr.get("ids").and_then(Value::as_array) {
                decoration_ids.extend(ids.iter().filter_map(Value::as_u64));
            }
        }
    } else if let Some(arr) = custom_decos {
        for item in arr {
            let mut req = item.clone();
            req["infographicId"] = json!(infographic_id);
            let dr = place_decoration(s, &req)?;
            if let Some(ids) = dr.get("ids").and_then(Value::as_array) {
                decoration_ids.extend(ids.iter().filter_map(Value::as_u64));
            }
        }
    }

    Ok(json!({
        "infographicId": infographic_id,
        "template": tpl.id,
        "chartId": chart_res["chartId"],
        "graphNodeId": chart_res["graphNodeId"],
        "overlayNodeIds": chart_res["overlayNodeIds"],
        "textNodeIds": text_ids,
        "decorationNodeIds": decoration_ids,
    }))
}
