use serde_json::{Map, Value, json};
use vectorcraft_doc::{Document, NodeId};

pub const META_KEY: &str = "infographic";
pub const SCHEMA_VERSION: u64 = 1;

pub fn read_doc_meta(doc: &Document) -> (Map<String, Value>, Vec<String>) {
    let mut warnings = Vec::new();
    let Some(raw) = doc.unknown.get(META_KEY) else {
        return (default_root(), warnings);
    };
    let Some(obj) = raw.as_object() else {
        warnings.push("infographic metadata is not an object; ignoring".into());
        return (default_root(), warnings);
    };
    let mut root = obj.clone();
    let version = root.get("schemaVersion").and_then(Value::as_u64).unwrap_or(0);
    if version != SCHEMA_VERSION {
        warnings.push(format!("infographic metadata schemaVersion {version} != {SCHEMA_VERSION}; best-effort read"));
        root.insert("schemaVersion".into(), json!(SCHEMA_VERSION));
    }
    if !root.get("infographics").is_some_and(Value::is_array) {
        root.insert("infographics".into(), json!([]));
    }
    (root, warnings)
}

pub fn write_doc_meta(doc: &mut Document, root: Map<String, Value>) {
    doc.unknown.insert(META_KEY.into(), Value::Object(root));
}

pub fn default_root() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("schemaVersion".into(), json!(SCHEMA_VERSION));
    m.insert("infographics".into(), json!([]));
    m
}

pub fn ensure_infographic<'a>(
    root: &'a mut Map<String, Value>,
    infographic_id: Option<&str>,
    artboard_id: Option<u32>,
) -> &'a mut Map<String, Value> {
    let infos = root.entry("infographics").or_insert_with(|| json!([]));
    if !infos.is_array() {
        *infos = json!([]);
    }
    let arr = infos.as_array_mut().unwrap();
    if let Some(id) = infographic_id {
        if let Some(pos) = arr.iter().position(|v| v.get("id").and_then(Value::as_str) == Some(id)) {
            return arr[pos].as_object_mut().unwrap();
        }
        let mut obj = Map::new();
        obj.insert("id".into(), json!(id));
        if let Some(a) = artboard_id {
            obj.insert("artboardId".into(), json!(a));
        }
        obj.insert("charts".into(), json!([]));
        obj.insert("objects".into(), json!([]));
        arr.push(Value::Object(obj));
        let last = arr.len() - 1;
        return arr[last].as_object_mut().unwrap();
    }
    if arr.is_empty() {
        let mut obj = Map::new();
        obj.insert("id".into(), json!("infographic-1"));
        if let Some(a) = artboard_id {
            obj.insert("artboardId".into(), json!(a));
        }
        obj.insert("charts".into(), json!([]));
        obj.insert("objects".into(), json!([]));
        arr.push(Value::Object(obj));
    }
    arr[0].as_object_mut().unwrap()
}

pub fn find_chart_mut<'a>(info: &'a mut Map<String, Value>, chart_id: &str) -> Option<&'a mut Map<String, Value>> {
    let charts = info.get_mut("charts")?.as_array_mut()?;
    charts.iter_mut().find_map(|v| {
        let obj = v.as_object_mut()?;
        (obj.get("id").and_then(Value::as_str) == Some(chart_id)).then_some(obj)
    })
}

pub fn push_chart(info: &mut Map<String, Value>, chart: Map<String, Value>) {
    let charts = info.entry("charts").or_insert_with(|| json!([]));
    if !charts.is_array() {
        *charts = json!([]);
    }
    charts.as_array_mut().unwrap().push(Value::Object(chart));
}

pub fn next_chart_id(info: &Map<String, Value>) -> String {
    let mut n = 1u64;
    let existing: std::collections::BTreeSet<String> = info
        .get("charts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect();
    loop {
        let id = format!("chart-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

pub fn node_exists(doc: &Document, id: u64) -> bool {
    doc.node(NodeId(id)).is_some()
}

pub fn chart_warnings(doc: &Document, chart: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::new();
    let cid = chart.get("id").and_then(Value::as_str).unwrap_or("<unknown>");
    for key in ["nodeId", "graphNodeId"] {
        if let Some(id) = chart.get(key).and_then(Value::as_u64) {
            if !node_exists(doc, id) {
                out.push(format!("{cid}: stale {key} {id}"));
            }
        }
    }
    for key in ["generatedNodeIds", "overlayNodeIds"] {
        if let Some(arr) = chart.get(key).and_then(Value::as_array) {
            for id in arr.iter().filter_map(Value::as_u64) {
                if !node_exists(doc, id) {
                    out.push(format!("{cid}: stale {key} node {id}"));
                }
            }
        }
    }
    out
}
