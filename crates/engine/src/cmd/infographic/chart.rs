use std::sync::Arc;

use serde_json::{Map, Value, json};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Appearance, GraphDesign, Node, NodeId, NodeKind};
use vectorcraft_geom::{BezPath, PathData, Rect};

use super::csv::{Delimiter, decode_csv_bytes, load_csv_input, parse_number, parse_rows};
use super::metadata::{
    SCHEMA_VERSION, chart_warnings, ensure_infographic, find_chart_mut, next_chart_id, push_chart, read_doc_meta, remove_chart_meta, write_doc_meta,
};
use super::presets::{default_preset, is_supported_preset};
use super::{bad_param, text_or};

const C_CREATE: &str = "infographic.createChart";
const C_ADD: &str = "infographic.addChartFromCsv";
const C_UPDATE: &str = "infographic.updateChart";

#[derive(Clone, Debug)]
struct Mapping {
    category: String,
    values: Vec<String>,
    series: Option<String>,
    missing: MissingPolicy,
    aggregation: Aggregation,
}

#[derive(Clone, Copy, Debug)]
enum MissingPolicy {
    Skip,
    Zero,
    Error,
}
#[derive(Clone, Copy, Debug)]
enum Aggregation {
    NoneAgg,
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

#[derive(Clone, Debug)]
struct ChartOptions {
    preset: String,
    y_axis: String,
    x_label_style: String,
    legend: bool,
    highlight_mode: String,
    highlight_index: Option<usize>,
    name: Option<String>,
}

pub fn list_charts(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let st = s.doc()?;
    let (root, mut warnings) = read_doc_meta(&st.doc);
    let infographic_id = text_or(p, "infographicId");
    let infos = root.get("infographics").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut charts_out = Vec::new();
    for info in infos.iter().filter_map(Value::as_object) {
        if let Some(want) = infographic_id {
            if info.get("id").and_then(Value::as_str) != Some(want) {
                continue;
            }
        }
        if let Some(arr) = info.get("charts").and_then(Value::as_array) {
            for chart in arr.iter().filter_map(Value::as_object) {
                warnings.extend(chart_warnings(&st.doc, chart));
                charts_out.push(Value::Object(chart.clone()));
            }
        }
    }
    Ok(json!({ "charts": charts_out, "warnings": warnings }))
}

pub fn create_chart(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    create_or_add(s, p, C_CREATE)
}

pub fn add_chart_from_csv(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    create_or_add(s, p, C_ADD)
}

fn create_or_add(s: &mut crate::Session, p: &Value, cmd: &str) -> crate::Result<Value> {
    let input = load_csv_input(p, cmd)?;
    let text = decode_csv_bytes(&input.bytes, cmd)?;
    let rows = parse_rows(&text, Delimiter::Auto);
    if rows.is_empty() {
        return Err(bad_param(cmd, "CSV has no rows"));
    }
    let mapping = parse_mapping(p, cmd)?;
    let opts = parse_options(p, cmd)?;
    let normalized = normalize_data(&rows, &mapping, cmd)?;
    let placement = parse_placement(p)?;
    validate_preset(&opts, cmd)?;
    let _ = resolve_highlight_target(&opts, &normalized, cmd)?;

    // Validation only above. Mutations below.
    let graph_csv = to_graph_csv(&normalized);
    let (x, y, w, h) = placement.unwrap_or((80.0, 80.0, 640.0, 360.0));
    let create_params = json!({
        "type": "column",
        "x": x, "y": y, "width": w, "height": h,
        "csv": graph_csv,
    });
    let created = s.execute("graph.create", &create_params)?;
    let graph_id = created.get("id").and_then(Value::as_u64).ok_or_else(|| bad_param(cmd, "graph.create did not return id"))?;

    let overlay_ids = apply_chart_style(s, graph_id, &opts, &normalized, cmd)?;

    let chart_id = write_chart_metadata(s, p, (&input.source, &text), &mapping, &opts, (graph_id, &overlay_ids), &normalized)?;

    Ok(json!({
        "chartId": chart_id,
        "graphNodeId": graph_id,
        "overlayNodeIds": overlay_ids,
        "preset": opts.preset,
        "rows": normalized.categories.len(),
        "series": normalized.series.len(),
    }))
}

pub fn update_chart(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let chart_id = text_or(p, "chartId").ok_or_else(|| bad_param(C_UPDATE, "missing `chartId`"))?;
    let st = s.doc()?;
    let (mut root, _warnings) = read_doc_meta(&st.doc);
    let infographic_id = text_or(p, "infographicId");
    let info = ensure_infographic(&mut root, infographic_id, None).ok_or_else(|| bad_param(C_UPDATE, "infographic metadata unavailable"))?;
    let existing = find_chart_mut(info, chart_id).ok_or_else(|| bad_param(C_UPDATE, format!("unknown chart `{chart_id}`")))?.clone();

    let mapping = if p.get("mapping").is_some() { parse_mapping(p, C_UPDATE)? } else { mapping_from_meta(&existing, C_UPDATE)? };
    let opts = parse_options_update(p, &existing, C_UPDATE)?;
    validate_preset(&opts, C_UPDATE)?;

    let (source, text, rows) = if has_csv_input(p) {
        let input = load_csv_input(p, C_UPDATE)?;
        let text = decode_csv_bytes(&input.bytes, C_UPDATE)?;
        let rows = parse_rows(&text, Delimiter::Auto);
        (input.source, text, rows)
    } else {
        let source = existing.get("source").and_then(|v| v.get("path")).and_then(Value::as_str).unwrap_or("inline").to_string();
        let csv = existing.get("source").and_then(|v| v.get("csv")).and_then(Value::as_str).unwrap_or("");
        let rows = if !csv.is_empty() {
            parse_rows(csv, Delimiter::Auto)
        } else {
            return Err(bad_param(C_UPDATE, "chart has no source CSV; provide csvPath/csvData"));
        };
        (source, csv.to_string(), rows)
    };
    if rows.is_empty() {
        return Err(bad_param(C_UPDATE, "CSV has no rows"));
    }
    let normalized = normalize_data(&rows, &mapping, C_UPDATE)?;
    let _ = resolve_highlight_target(&opts, &normalized, C_UPDATE)?;

    let graph_id = existing
        .get("graphNodeId")
        .or_else(|| existing.get("nodeId"))
        .and_then(Value::as_u64)
        .ok_or_else(|| bad_param(C_UPDATE, "chart metadata missing graphNodeId"))?;
    if st.doc.node(NodeId(graph_id)).is_none() {
        return Err(bad_param(C_UPDATE, format!("stale graphNodeId {graph_id}")));
    }

    // Validation done. Mutate.
    cleanup_overlays(s, &existing);
    let graph_csv = to_graph_csv(&normalized);
    s.execute("graph.setData", &json!({"id": graph_id, "csv": graph_csv}))?;
    let overlay_ids = apply_chart_style(s, graph_id, &opts, &normalized, C_UPDATE)?;

    let st2 = s.doc()?;
    let (mut root2, _) = read_doc_meta(&st2.doc);
    let info2 = ensure_infographic(&mut root2, infographic_id, None).ok_or_else(|| bad_param(C_UPDATE, "infographic metadata unavailable"))?;
    let chart = find_chart_mut(info2, chart_id).ok_or_else(|| bad_param(C_UPDATE, "chart metadata missing after update"))?;
    chart.insert("source".into(), json!({"kind":"csv","path":source, "csv": if has_csv_input(p) { json!(text) } else { existing.get("source").and_then(|v| v.get("csv")).cloned().unwrap_or(json!("")) }}));
    chart.insert("mapping".into(), mapping_to_json(&mapping));
    chart.insert("preset".into(), json!(opts.preset));
    chart.insert("axis".into(), json!({"y":opts.y_axis,"xLabelStyle":opts.x_label_style,"legend":opts.legend}));
    chart.insert("highlight".into(), json!({"mode":opts.highlight_mode,"index":opts.highlight_index}));
    if let Some(name) = opts.name {
        chart.insert("name".into(), json!(name));
    }
    chart.insert("graphNodeId".into(), json!(graph_id));
    chart.insert("nodeId".into(), json!(graph_id));
    chart.insert("schemaVersion".into(), json!(SCHEMA_VERSION));
    let mut generated = vec![graph_id];
    generated.extend(&overlay_ids);
    chart.insert("overlayNodeIds".into(), json!(overlay_ids));
    chart.insert("generatedNodeIds".into(), json!(generated));
    let st2m = s.doc_mut()?;
    write_doc_meta(Arc::make_mut(&mut st2m.doc), root2);

    Ok(json!({"chartId": chart_id, "graphNodeId": graph_id, "overlayNodeIds": overlay_ids, "preset": opts.preset, "updated": true}))
}

pub fn remove_chart(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    const C_REMOVE: &str = "infographic.removeChart";
    let chart_id = text_or(p, "chartId").ok_or_else(|| bad_param(C_REMOVE, "missing `chartId`"))?;
    let infographic_id = text_or(p, "infographicId");
    let st = s.doc()?;
    let (mut root, _) = read_doc_meta(&st.doc);
    let info = ensure_infographic(&mut root, infographic_id, None).ok_or_else(|| bad_param(C_UPDATE, "infographic metadata unavailable"))?;
    let removed = remove_chart_meta(info, chart_id).ok_or_else(|| bad_param(C_REMOVE, format!("unknown chart `{chart_id}`")))?;

    let mut ids_to_delete: Vec<u64> = Vec::new();
    for key in ["graphNodeId", "nodeId"] {
        if let Some(id) = removed.get(key).and_then(Value::as_u64) {
            if !ids_to_delete.contains(&id) {
                ids_to_delete.push(id);
            }
        }
    }
    for key in ["overlayNodeIds", "generatedNodeIds"] {
        if let Some(arr) = removed.get(key).and_then(Value::as_array) {
            for id in arr.iter().filter_map(Value::as_u64) {
                if !ids_to_delete.contains(&id) {
                    ids_to_delete.push(id);
                }
            }
        }
    }

    s.edit("Remove Infographic Chart", |doc, sel| {
        for id in &ids_to_delete {
            let nid = NodeId(*id);
            if doc.node(nid).is_some() {
                let _ = doc.remove(nid);
                sel.objects.retain(|x| *x != nid);
            }
        }
        write_doc_meta(doc, root);
        Ok(())
    })?;

    Ok(json!({"chartId": chart_id, "removed": true, "removedNodeIds": ids_to_delete}))
}

fn has_csv_input(p: &Value) -> bool {
    text_or(p, "csvPath")
        .or(text_or(p, "path"))
        .or(text_or(p, "csvData"))
        .or(text_or(p, "data"))
        .or(text_or(p, "csv"))
        .or(text_or(p, "csvBase64"))
        .or(text_or(p, "dataBase64"))
        .is_some()
}

fn parse_mapping(p: &Value, cmd: &str) -> crate::Result<Mapping> {
    let m = p.get("mapping").ok_or_else(|| bad_param(cmd, "missing `mapping`"))?;
    let category = m
        .get("category")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_param(cmd, "mapping.category is required"))?
        .to_string();
    let values = m
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| bad_param(cmd, "mapping.values must be an array"))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(bad_param(cmd, "mapping.values must contain at least one column"));
    }
    let series = m.get("series").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    let missing = match m.get("missing").and_then(Value::as_str).unwrap_or("skip") {
        "skip" => MissingPolicy::Skip,
        "zero" => MissingPolicy::Zero,
        "error" => MissingPolicy::Error,
        other => return Err(bad_param(cmd, format!("unknown mapping.missing `{other}`"))),
    };
    let aggregation = match m.get("aggregation").and_then(Value::as_str).unwrap_or("none") {
        "none" => Aggregation::NoneAgg,
        "sum" => Aggregation::Sum,
        "avg" => Aggregation::Avg,
        "count" => Aggregation::Count,
        "min" => Aggregation::Min,
        "max" => Aggregation::Max,
        other => return Err(bad_param(cmd, format!("unknown mapping.aggregation `{other}`"))),
    };
    Ok(Mapping { category, values, series, missing, aggregation })
}

fn mapping_from_meta(chart: &Map<String, Value>, cmd: &str) -> crate::Result<Mapping> {
    let m = chart.get("mapping").cloned().ok_or_else(|| bad_param(cmd, "chart metadata missing mapping"))?;
    parse_mapping(&json!({"mapping": m}), cmd)
}

fn mapping_to_json(m: &Mapping) -> Value {
    json!({
        "category": m.category,
        "values": m.values,
        "series": m.series,
        "missing": match m.missing { MissingPolicy::Skip=>"skip", MissingPolicy::Zero=>"zero", MissingPolicy::Error=>"error" },
        "aggregation": match m.aggregation { Aggregation::NoneAgg=>"none", Aggregation::Sum=>"sum", Aggregation::Avg=>"avg", Aggregation::Count=>"count", Aggregation::Min=>"min", Aggregation::Max=>"max" },
    })
}

fn parse_options(p: &Value, cmd: &str) -> crate::Result<ChartOptions> {
    let preset = text_or(p, "preset").or_else(|| p.get("stylePreset").and_then(Value::as_str)).unwrap_or(default_preset()).to_string();
    let axis = p.get("axis");
    let y_axis = axis.and_then(|a| a.get("y")).and_then(Value::as_str).unwrap_or("minimal").to_string();
    let x_label_style = axis.and_then(|a| a.get("xLabelStyle")).and_then(Value::as_str).unwrap_or("bold").to_string();
    let legend = axis.and_then(|a| a.get("legend")).and_then(Value::as_bool).unwrap_or(false);
    let h = p.get("highlight");
    let highlight_mode = h.and_then(|v| v.get("mode")).and_then(Value::as_str).unwrap_or("none").to_string();
    let highlight_index = h.and_then(|v| v.get("index")).and_then(Value::as_u64).map(|v| v as usize);
    let name = text_or(p, "name").map(str::to_owned);
    let opts = ChartOptions { preset, y_axis, x_label_style, legend, highlight_mode, highlight_index, name };
    validate_options(&opts, cmd)?;
    Ok(opts)
}

fn parse_options_update(p: &Value, existing: &Map<String, Value>, cmd: &str) -> crate::Result<ChartOptions> {
    let preset = text_or(p, "preset")
        .map(str::to_owned)
        .or_else(|| existing.get("preset").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| default_preset().to_string());
    let axis = p.get("axis");
    let ex_axis = existing.get("axis");
    let y_axis = axis
        .and_then(|a| a.get("y"))
        .and_then(Value::as_str)
        .or_else(|| ex_axis.and_then(|a| a.get("y")).and_then(Value::as_str))
        .unwrap_or("minimal")
        .to_string();
    let x_label_style = axis
        .and_then(|a| a.get("xLabelStyle"))
        .and_then(Value::as_str)
        .or_else(|| ex_axis.and_then(|a| a.get("xLabelStyle")).and_then(Value::as_str))
        .unwrap_or("bold")
        .to_string();
    let legend = axis
        .and_then(|a| a.get("legend"))
        .and_then(Value::as_bool)
        .or_else(|| ex_axis.and_then(|a| a.get("legend")).and_then(Value::as_bool))
        .unwrap_or(false);
    let h = p.get("highlight");
    let ex_h = existing.get("highlight");
    let highlight_mode = h
        .and_then(|v| v.get("mode"))
        .and_then(Value::as_str)
        .or_else(|| ex_h.and_then(|v| v.get("mode")).and_then(Value::as_str))
        .unwrap_or("none")
        .to_string();
    let highlight_index = h
        .and_then(|v| v.get("index"))
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .or_else(|| ex_h.and_then(|v| v.get("index")).and_then(Value::as_u64).map(|v| v as usize));
    let name = text_or(p, "name").map(str::to_owned).or_else(|| existing.get("name").and_then(Value::as_str).map(str::to_owned));
    let opts = ChartOptions { preset, y_axis, x_label_style, legend, highlight_mode, highlight_index, name };
    validate_options(&opts, cmd)?;
    Ok(opts)
}

fn validate_options(opts: &ChartOptions, cmd: &str) -> crate::Result<()> {
    if !is_supported_preset(&opts.preset) {
        return Err(bad_param(cmd, format!("unknown preset `{}`", opts.preset)));
    }
    match opts.y_axis.as_str() {
        "minimal" | "hidden" => {}
        other => return Err(bad_param(cmd, format!("unknown axis.y `{other}`"))),
    }
    match opts.x_label_style.as_str() {
        "bold" | "normal" => {}
        other => return Err(bad_param(cmd, format!("unknown axis.xLabelStyle `{other}`"))),
    }
    match opts.highlight_mode.as_str() {
        "none" | "last" | "max" | "min" | "index" => {}
        other => return Err(bad_param(cmd, format!("unknown highlight.mode `{other}`"))),
    }
    if opts.highlight_mode == "index" && opts.highlight_index.is_none() {
        return Err(bad_param(cmd, "highlight.index required when highlight.mode=index"));
    }
    Ok(())
}

fn validate_preset(opts: &ChartOptions, cmd: &str) -> crate::Result<()> {
    if opts.preset == "bar-building" && opts.highlight_mode != "none" {
        return Err(bad_param(cmd, "bar-building preset does not support highlight modes in MVP"));
    }
    Ok(())
}

fn parse_placement(p: &Value) -> crate::Result<Option<(f64, f64, f64, f64)>> {
    let Some(pl) = p.get("placement") else {
        return Ok(None);
    };
    let x = pl.get("x").and_then(Value::as_f64).unwrap_or(80.0);
    let y = pl.get("y").and_then(Value::as_f64).unwrap_or(80.0);
    let w = pl.get("width").and_then(Value::as_f64).unwrap_or(640.0).max(1.0);
    let h = pl.get("height").and_then(Value::as_f64).unwrap_or(360.0).max(1.0);
    Ok(Some((x, y, w, h)))
}

struct NormalizedData {
    categories: Vec<String>,
    series: Vec<String>,
    rows: Vec<Vec<Option<f64>>>,
}

fn normalize_data(rows: &[Vec<String>], mapping: &Mapping, cmd: &str) -> crate::Result<NormalizedData> {
    let header = &rows[0];
    let idx = |name: &str| -> crate::Result<usize> {
        header.iter().position(|h| h.trim() == name).ok_or_else(|| bad_param(cmd, format!("missing CSV column `{name}`")))
    };
    let cat_idx = idx(&mapping.category)?;
    let value_idxs = mapping.values.iter().map(|v| idx(v)).collect::<crate::Result<Vec<_>>>()?;
    let series_idx = mapping.series.as_ref().map(|s| idx(s)).transpose()?;

    let mut categories: Vec<String> = Vec::new();
    let mut cat_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut series_names: Vec<String> = Vec::new();
    let mut ser_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    // Determine categories and series
    if let Some(sidx) = series_idx {
        // pivot: mapping.values must have one value column
        if mapping.values.len() != 1 {
            return Err(bad_param(cmd, "mapping.series pivot requires exactly one mapping.values column"));
        }
        for r in rows.iter().skip(1) {
            let cat = r.get(cat_idx).cloned().unwrap_or_default().trim().to_string();
            if cat.is_empty() {
                continue;
            }
            if !cat_map.contains_key(&cat) {
                cat_map.insert(cat.clone(), categories.len());
                categories.push(cat.clone());
            }
            let ser = r.get(sidx).cloned().unwrap_or_else(|| "Series".into()).trim().to_string();
            let ser = if ser.is_empty() { "Series".into() } else { ser };
            if !ser_map.contains_key(&ser) {
                ser_map.insert(ser.clone(), series_names.len());
                series_names.push(ser.clone());
            }
        }
        let mut table = vec![vec![Vec::<f64>::new(); series_names.len()]; categories.len()];
        for r in rows.iter().skip(1) {
            let cat = r.get(cat_idx).cloned().unwrap_or_default().trim().to_string();
            if cat.is_empty() {
                continue;
            }
            let ser = r.get(sidx).cloned().unwrap_or_else(|| "Series".into()).trim().to_string();
            let ser = if ser.is_empty() { "Series".into() } else { ser };
            let Some(ci) = cat_map.get(&cat).copied() else {
                continue;
            };
            let Some(si) = ser_map.get(&ser).copied() else {
                continue;
            };
            let raw = r.get(value_idxs[0]).cloned().unwrap_or_default();
            match cell_to_num(&raw, mapping.missing, cmd)? {
                Some(v) => table[ci][si].push(v),
                None => {}
            }
        }
        let rows_out =
            table.into_iter().map(|row| row.into_iter().enumerate().map(|(_i, vals)| aggregate(&vals, mapping.aggregation)).collect()).collect();
        return Ok(NormalizedData { categories, series: series_names, rows: rows_out });
    }

    for r in rows.iter().skip(1) {
        let cat = r.get(cat_idx).cloned().unwrap_or_default().trim().to_string();
        if cat.is_empty() {
            continue;
        }
        if !cat_map.contains_key(&cat) {
            cat_map.insert(cat.clone(), categories.len());
            categories.push(cat.clone());
        }
    }
    series_names = mapping.values.clone();
    let mut table = vec![vec![Vec::<f64>::new(); series_names.len()]; categories.len()];
    for r in rows.iter().skip(1) {
        let cat = r.get(cat_idx).cloned().unwrap_or_default().trim().to_string();
        if cat.is_empty() {
            continue;
        }
        let Some(ci) = cat_map.get(&cat).copied() else {
            continue;
        };
        for (si, vidx) in value_idxs.iter().enumerate() {
            let raw = r.get(*vidx).cloned().unwrap_or_default();
            match cell_to_num(&raw, mapping.missing, cmd)? {
                Some(v) => table[ci][si].push(v),
                None => {}
            }
        }
    }
    let rows_out = table.into_iter().map(|row| row.into_iter().map(|vals| aggregate(&vals, mapping.aggregation)).collect()).collect();
    Ok(NormalizedData { categories, series: series_names, rows: rows_out })
}

fn cell_to_num(raw: &str, missing: MissingPolicy, cmd: &str) -> crate::Result<Option<f64>> {
    let t = raw.trim();
    if t.is_empty() {
        return match missing {
            MissingPolicy::Skip => Ok(None),
            MissingPolicy::Zero => Ok(Some(0.0)),
            MissingPolicy::Error => Err(bad_param(cmd, "missing numeric value")),
        };
    }
    match parse_number(t) {
        Some(v) => Ok(Some(v)),
        None => match missing {
            MissingPolicy::Skip => Ok(None),
            MissingPolicy::Zero => Ok(Some(0.0)),
            MissingPolicy::Error => Err(bad_param(cmd, format!("invalid numeric value `{t}`"))),
        },
    }
}

fn aggregate(vals: &[f64], agg: Aggregation) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    Some(match agg {
        Aggregation::NoneAgg => *vals.last().unwrap(),
        Aggregation::Sum => vals.iter().sum(),
        Aggregation::Avg => vals.iter().sum::<f64>() / vals.len() as f64,
        Aggregation::Count => vals.len() as f64,
        Aggregation::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
        Aggregation::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    })
}

fn to_graph_csv(data: &NormalizedData) -> String {
    let mut out = String::new();
    out.push_str("Category");
    for s in &data.series {
        out.push(',');
        out.push_str(&csv_escape(s));
    }
    out.push('\n');
    for (ci, cat) in data.categories.iter().enumerate() {
        out.push_str(&csv_escape(cat));
        for si in 0..data.series.len() {
            out.push(',');
            if let Some(v) = data.rows.get(ci).and_then(|r| r.get(si)).copied().flatten() {
                out.push_str(&format!("{}", v));
            }
        }
        out.push('\n');
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

fn apply_chart_style(s: &mut crate::Session, graph_id: u64, opts: &ChartOptions, normalized: &NormalizedData, cmd: &str) -> crate::Result<Vec<u64>> {
    let set_type = json!({
        "id": graph_id,
        "type": "column",
        "legend": opts.legend,
        "tickLength": "none",
        "tickMarks": 1,
        "prefix": "",
        "suffix": "",
    });
    s.execute("graph.setType", &set_type)?;

    if opts.x_label_style == "bold" {
        let _ = apply_bold_category_labels(s, graph_id);
    }

    match opts.preset.as_str() {
        "bar-building" => apply_column_design_preset(s, graph_id, "Infographic Building", build_building_design_art)?,
        "bar-isometric-blue" => apply_column_design_preset(s, graph_id, "Infographic Isometric Blue", build_isometric_blue_design_art)?,
        "bar-stepped-ribbon" => apply_column_design_preset(s, graph_id, "Infographic Stepped Ribbon", build_stepped_ribbon_design_art)?,
        "bar-decline-green-blue" => apply_column_design_preset(s, graph_id, "Infographic Green Blue", build_green_blue_design_art)?,
        "bar-isometric-orange" => apply_column_design_preset(s, graph_id, "Infographic Isometric Orange", build_isometric_orange_design_art)?,
        _ => {}
    }

    let mut overlay_ids = Vec::new();
    if opts.preset == "bar-arrow-highlight" && opts.highlight_mode != "none" {
        if let Some(id) = apply_arrow_highlight(s, graph_id, opts, normalized, cmd)? {
            overlay_ids.push(id);
        }
    }
    Ok(overlay_ids)
}

fn apply_bold_category_labels(s: &mut crate::Session, graph_id: u64) -> crate::Result<()> {
    let root = NodeId(graph_id);
    s.edit("Infographic Labels", |doc, _sel| {
        let Some(node) = doc.node(root).cloned() else {
            return Ok(());
        };
        let mut stack = vec![node];
        let mut ids = Vec::new();
        while let Some(n) = stack.pop() {
            if let vectorcraft_doc::NodeKind::Text(t) = &n.kind {
                let content = t.plain_text();
                // Heuristic: category labels are not numeric and not series labels in legend; graph text roles are not exposed directly.
                if !content.trim().is_empty() && parse_number(content.trim()).is_none() {
                    ids.push(n.id);
                }
            }
            if let Some(ch) = n.children() {
                for c in ch.iter().rev() {
                    stack.push(c.as_ref().clone());
                }
            }
        }
        for id in ids {
            if let Some(n) = doc.node_mut(id) {
                if let vectorcraft_doc::NodeKind::Text(t) = &mut n.kind {
                    for r in &mut t.runs {
                        if r.style.font_style.eq_ignore_ascii_case("Regular") || r.style.font_style.is_empty() {
                            r.style.font_style = "Bold".into();
                        }
                        r.style.font_axes.insert("wght".into(), 700.0);
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(())
}

fn apply_arrow_highlight(
    s: &mut crate::Session,
    graph_id: u64,
    opts: &ChartOptions,
    normalized: &NormalizedData,
    cmd: &str,
) -> crate::Result<Option<u64>> {
    let target = resolve_highlight_target(opts, normalized, cmd)?;
    let Some(target_idx) = target else {
        return Ok(None);
    };
    let bar = find_target_bar_rect(s, graph_id, target_idx, normalized.categories.len())?;
    let Some(rect) = bar else {
        return Ok(None);
    };

    let overlay_id = s.edit("Infographic Highlight", |doc, _sel| {
        let arrow = make_arrow_overlay(doc, rect);
        let id = arrow.id;
        let gid = NodeId(graph_id);
        let parent = doc.parent_of(gid);
        if let Some(pid) = parent {
            if let Some(p) = doc.node_mut(pid) {
                if let Some(children) = p.children_mut() {
                    if let Some(pos) = children.iter().position(|c| c.id == gid) {
                        children.insert(pos + 1, Arc::new(arrow));
                    } else {
                        children.push(Arc::new(arrow));
                    }
                }
            }
        } else {
            if let Some(layer) = doc.layers.first_mut() {
                let layer = Arc::make_mut(layer);
                if let Some(children) = layer.children_mut() {
                    children.push(Arc::new(arrow));
                }
            }
        }
        Ok(id)
    })?;

    Ok(Some(overlay_id.0))
}

fn cleanup_overlays(s: &mut crate::Session, chart: &Map<String, Value>) {
    let Some(arr) = chart.get("overlayNodeIds").and_then(Value::as_array) else {
        return;
    };
    let ids: Vec<u64> = arr.iter().filter_map(Value::as_u64).collect();
    if ids.is_empty() {
        return;
    }
    let _ = s.edit("Infographic Cleanup", |doc, sel| {
        for id in ids {
            let nid = NodeId(id);
            if doc.node(nid).is_some() {
                let _ = doc.remove(nid);
                sel.objects.retain(|x| *x != nid);
            }
        }
        Ok(())
    });
}

fn resolve_highlight_target(opts: &ChartOptions, normalized: &NormalizedData, cmd: &str) -> crate::Result<Option<usize>> {
    let n = normalized.categories.len();
    if n == 0 {
        return Ok(None);
    }
    let idx = match opts.highlight_mode.as_str() {
        "none" => return Ok(None),
        "last" => n.saturating_sub(1),
        "index" => {
            let i = opts.highlight_index.ok_or_else(|| bad_param(cmd, "highlight.index required"))?;
            if i >= n {
                return Err(bad_param(cmd, format!("highlight.index {i} out of range (0..{n})")));
            }
            i
        }
        "max" | "min" => {
            let mut best_i = 0usize;
            let mut best_v = if opts.highlight_mode == "max" { f64::NEG_INFINITY } else { f64::INFINITY };
            for i in 0..n {
                let v = normalized.rows.get(i).and_then(|r| r.first()).copied().flatten().unwrap_or(f64::NAN);
                if !v.is_finite() {
                    continue;
                }
                let better = if opts.highlight_mode == "max" { v > best_v } else { v < best_v };
                if better {
                    best_v = v;
                    best_i = i;
                }
            }
            if !best_v.is_finite() {
                return Ok(None);
            }
            best_i
        }
        _ => return Ok(None),
    };
    Ok(Some(idx))
}

fn find_target_bar_rect(s: &crate::Session, graph_id: u64, target_idx: usize, cat_count: usize) -> crate::Result<Option<Rect>> {
    let st = s.doc()?;
    let root = st.doc.node(NodeId(graph_id)).ok_or_else(|| bad_param("infographic.updateChart", "graph node missing"))?;
    let Some(series_groups) = root.children() else {
        return Ok(None);
    };

    let mut candidates: Vec<(u32, Rect)> = Vec::new();
    for g in series_groups {
        let Some(si) = g.extras().series_index else { continue };
        if si != 0 {
            continue; // MVP: primary series
        }
        let mut stack = vec![g.as_ref()];
        while let Some(n) = stack.pop() {
            if let NodeKind::Path { .. } = &n.kind {
                // skip obvious non-bar paths? use bounds; bar rects are axis-aligned-ish in generated graph
                if let Some(b) = n.geometric_bounds() {
                    if b.width() > 1.0 && b.height() > 1.0 {
                        candidates.push((si, b));
                    }
                }
            }
            if let Some(ch) = n.children() {
                for c in ch.iter().rev() {
                    stack.push(c.as_ref());
                }
            }
        }
    }

    if candidates.is_empty() || cat_count == 0 {
        return Ok(None);
    }

    // Group by x-center proximity into category buckets.
    candidates.sort_by(|a, b| a.1.center().x.partial_cmp(&b.1.center().x).unwrap_or(std::cmp::Ordering::Equal));
    let mut buckets: Vec<Vec<Rect>> = Vec::new();
    for (_, r) in candidates {
        let cx = r.center().x;
        let same = buckets.last_mut().filter(|b| {
            let bx = b.iter().map(|x| x.center().x).sum::<f64>() / b.len() as f64;
            (cx - bx).abs() < (r.width().max(8.0) * 0.75)
        });
        if let Some(b) = same {
            b.push(r);
        } else {
            buckets.push(vec![r]);
        }
    }

    // If bucketing misfires, fall back to simple left-to-right order.
    if buckets.len() < cat_count {
        // flatten and pick by sorted x, then target_idx clamped
        let mut all: Vec<Rect> = buckets.into_iter().flatten().collect();
        all.sort_by(|a, b| a.center().x.partial_cmp(&b.center().x).unwrap_or(std::cmp::Ordering::Equal));
        return Ok(all.get(target_idx).copied());
    }

    let bucket = buckets.get(target_idx.min(buckets.len() - 1)).ok_or_else(|| bad_param("infographic.updateChart", "highlight target not found"))?;
    // Choose tallest/most prominent rect in bucket.
    let best = bucket.iter().copied().max_by(|a, b| a.height().partial_cmp(&b.height()).unwrap_or(std::cmp::Ordering::Equal));
    Ok(best)
}

fn make_arrow_overlay(doc: &mut vectorcraft_doc::Document, bar: Rect) -> Node {
    let cx = bar.center().x;
    let top = bar.y0;
    let size = (bar.width().abs().max(18.0) * 0.8).clamp(18.0, 56.0);
    let gap = 8.0;
    let tip_y = top - gap;
    let base_y = tip_y - size;
    let half = size * 0.55;
    let mut bp = BezPath::new();
    bp.move_to((cx, tip_y));
    bp.line_to((cx - half, base_y));
    bp.line_to((cx + half, base_y));
    bp.close_path();
    let path = PathData::from_bezpath(&bp);
    let mut n = Node::path(doc.alloc_id(), path, Appearance::basic(Paint::solid(Color::rgb(0.95, 0.35, 0.1)), Paint::None, 0.0));
    n.name = Some("Infographic Highlight".into());
    n
}

fn write_chart_metadata(
    s: &mut crate::Session,
    p: &Value,
    (source, csv_text): (&str, &str),
    mapping: &Mapping,
    opts: &ChartOptions,
    (graph_id, overlay_ids): (u64, &[u64]),
    normalized: &NormalizedData,
) -> crate::Result<String> {
    let infographic_id = text_or(p, "infographicId");
    let st = s.doc_mut()?;
    let (mut root, _) = read_doc_meta(&st.doc);
    let info = ensure_infographic(
        &mut root,
        infographic_id,
        p.get("placement").and_then(|pl| pl.get("artboardId")).and_then(Value::as_u64).map(|v| v as u32),
    )
    .ok_or_else(|| bad_param("infographic.createChart", "infographic metadata unavailable"))?;
    let chart_id = next_chart_id(info);
    let mut chart = Map::new();
    chart.insert("id".into(), json!(chart_id));
    chart.insert("schemaVersion".into(), json!(SCHEMA_VERSION));
    chart.insert("source".into(), json!({"kind":"csv", "path": source, "csv": csv_text}));
    chart.insert("mapping".into(), mapping_to_json(mapping));
    chart.insert("preset".into(), json!(opts.preset));
    chart.insert("axis".into(), json!({"y":opts.y_axis,"xLabelStyle":opts.x_label_style,"legend":opts.legend}));
    chart.insert("highlight".into(), json!({"mode":opts.highlight_mode,"index":opts.highlight_index}));
    if let Some(name) = &opts.name {
        chart.insert("name".into(), json!(name));
    }
    let mut generated = vec![graph_id];
    generated.extend_from_slice(overlay_ids);
    chart.insert("graphNodeId".into(), json!(graph_id));
    chart.insert("nodeId".into(), json!(graph_id));
    chart.insert("generatedNodeIds".into(), json!(generated));
    chart.insert("overlayNodeIds".into(), json!(overlay_ids));
    chart.insert("dataSummary".into(), json!({"categories": normalized.categories.len(), "series": normalized.series.len()}));
    push_chart(info, chart);
    let chart_id = info
        .get("charts")
        .and_then(Value::as_array)
        .and_then(|a| a.last())
        .and_then(|v| v.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("chart-1")
        .to_string();
    write_doc_meta(Arc::make_mut(&mut st.doc), root);
    Ok(chart_id)
}
fn apply_column_design_preset(s: &mut crate::Session, graph_id: u64, design_name: &str, build_art: fn() -> Node) -> crate::Result<()> {
    ensure_column_design(s, design_name, build_art)?;
    s.execute(
        "graph.column",
        &json!({
            "id": graph_id,
            "design": design_name,
            "columnType": "sliding",
            "fractions": "chop"
        }),
    )?;
    Ok(())
}

fn ensure_column_design(s: &mut crate::Session, name: &str, build_art: fn() -> Node) -> crate::Result<()> {
    {
        let st = s.doc()?;
        if st.doc.graph_designs.iter().any(|d| d.name == name) {
            return Ok(());
        }
    }

    let art = build_art();
    s.edit("Infographic Design", |doc, _sel| {
        if doc.graph_designs.iter().any(|d| d.name == name) {
            return Ok(());
        }
        let art = doc.reid(&art);
        doc.graph_designs.push(GraphDesign { name: name.to_string(), art: Arc::new(art) });
        Ok(())
    })?;
    Ok(())
}

fn build_isometric_blue_design_art() -> Node {
    let frame = node_path("M 0 0 L 100 0 L 100 200 L 0 200 Z", Paint::None, Paint::None, 0.0, false);
    let left_face = node_path("M 0 26 L 50 52 L 50 200 L 0 174 Z", Paint::solid(Color::rgb8(92, 122, 161)), Paint::None, 0.0, false);
    let right_face = node_path("M 50 52 L 100 26 L 100 174 L 50 200 Z", Paint::solid(Color::rgb8(52, 85, 128)), Paint::None, 0.0, false);
    let top_cap = node_path("M 50 0 L 100 26 L 50 52 L 0 26 Z", Paint::solid(Color::rgb8(126, 151, 182)), Paint::None, 0.0, false);
    let guide = node_path("M 0 60 L 100 60", Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 0.0, true);
    Node::group(NodeId(0), vec![Arc::new(frame), Arc::new(left_face), Arc::new(right_face), Arc::new(top_cap), Arc::new(guide)])
}

fn build_stepped_ribbon_design_art() -> Node {
    let frame = node_path("M 0 0 L 100 0 L 100 200 L 0 200 Z", Paint::None, Paint::None, 0.0, false);
    let body = node_path("M 0 24 L 100 24 L 100 200 L 0 200 Z", Paint::solid(Color::rgb8(229, 135, 101)), Paint::None, 0.0, false);
    let fold = node_path("M 0 24 L 22 0 L 100 0 L 100 24 Z", Paint::solid(Color::rgb8(96, 125, 155)), Paint::None, 0.0, false);
    let accent = node_path("M 70 24 L 100 24 L 100 200 L 70 200 Z", Paint::solid(Color::rgb8(51, 93, 143)), Paint::None, 0.0, false);
    let guide = node_path("M 0 28 L 100 28", Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 0.0, true);
    Node::group(NodeId(0), vec![Arc::new(frame), Arc::new(body), Arc::new(accent), Arc::new(fold), Arc::new(guide)])
}

fn build_green_blue_design_art() -> Node {
    let frame = node_path("M 0 0 L 100 0 L 100 200 L 0 200 Z", Paint::None, Paint::None, 0.0, false);
    let left = node_path("M 0 16 L 68 16 L 68 200 L 0 200 Z", Paint::solid(Color::rgb8(46, 158, 143)), Paint::None, 0.0, false);
    let right = node_path("M 68 16 L 100 16 L 100 200 L 68 200 Z", Paint::solid(Color::rgb8(31, 111, 139)), Paint::None, 0.0, false);
    let cap = node_path("M 0 0 L 100 0 L 100 16 L 0 16 Z", Paint::solid(Color::rgb8(102, 194, 178)), Paint::None, 0.0, false);
    let guide = node_path("M 0 22 L 100 22", Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 0.0, true);
    Node::group(NodeId(0), vec![Arc::new(frame), Arc::new(left), Arc::new(right), Arc::new(cap), Arc::new(guide)])
}

fn build_isometric_orange_design_art() -> Node {
    let frame = node_path("M 0 0 L 100 0 L 100 200 L 0 200 Z", Paint::None, Paint::None, 0.0, false);
    let left = node_path("M 14 34 L 50 54 L 50 200 L 14 180 Z", Paint::solid(Color::rgb8(245, 158, 11)), Paint::None, 0.0, false);
    let right = node_path("M 50 54 L 86 34 L 86 180 L 50 200 Z", Paint::solid(Color::rgb8(217, 119, 6)), Paint::None, 0.0, false);
    let arrow_cap = node_path("M 50 0 L 100 34 L 50 54 L 0 34 Z", Paint::solid(Color::rgb8(251, 191, 36)), Paint::None, 0.0, false);
    let guide = node_path("M 0 60 L 100 60", Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 0.0, true);
    Node::group(NodeId(0), vec![Arc::new(frame), Arc::new(left), Arc::new(right), Arc::new(arrow_cap), Arc::new(guide)])
}

fn build_building_design_art() -> Node {
    let frame = node_path("M 0 0 L 100 0 L 100 200 L 0 200 Z", Paint::None, Paint::None, 0.0, false);
    let body = node_path("M 0 20 L 100 20 L 100 200 L 0 200 Z", Paint::solid(Color::rgb(0.23, 0.52, 0.86)), Paint::None, 0.0, false);
    let roof = node_path("M 0 20 L 50 0 L 100 20 Z", Paint::solid(Color::rgb(0.18, 0.43, 0.74)), Paint::None, 0.0, false);
    let win1 = node_path("M 20 50 L 35 50 L 35 65 L 20 65 Z", Paint::solid(Color::rgb(0.93, 0.96, 1.0)), Paint::None, 0.0, false);
    let win2 = node_path("M 65 50 L 80 50 L 80 65 L 65 65 Z", Paint::solid(Color::rgb(0.93, 0.96, 1.0)), Paint::None, 0.0, false);
    let win3 = node_path("M 20 90 L 35 90 L 35 105 L 20 105 Z", Paint::solid(Color::rgb(0.93, 0.96, 1.0)), Paint::None, 0.0, false);
    let win4 = node_path("M 65 90 L 80 90 L 80 105 L 65 105 Z", Paint::solid(Color::rgb(0.93, 0.96, 1.0)), Paint::None, 0.0, false);
    let guide = node_path("M 0 20 L 100 20", Paint::None, Paint::solid(Color::rgb(0.0, 0.0, 0.0)), 0.0, true);

    Node::group(
        NodeId(0),
        vec![Arc::new(frame), Arc::new(body), Arc::new(roof), Arc::new(win1), Arc::new(win2), Arc::new(win3), Arc::new(win4), Arc::new(guide)],
    )
}

fn node_path(d: &str, fill: Paint, stroke: Paint, stroke_w: f64, guide: bool) -> Node {
    let bp = BezPath::from_svg(d).unwrap_or_else(|_| BezPath::new());
    let pd = PathData::from_bezpath(&bp);
    let mut node = Node::path(NodeId(0), pd, Appearance::basic(fill, stroke, stroke_w));
    if guide {
        if let NodeKind::Path { guide: g, .. } = &mut node.kind {
            *g = true;
        }
    }
    node
}
