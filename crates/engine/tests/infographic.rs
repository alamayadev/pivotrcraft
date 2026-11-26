//! Infographic command integration tests (MVP).

use serde_json::json;
use vectorcraft_engine::Session;

fn id(v: &serde_json::Value) -> u64 {
    v["id"].as_u64().or_else(|| v["graphNodeId"].as_u64()).unwrap()
}

#[test]
fn inspect_csv_returns_columns_and_rows() {
    let mut s = Session::new();
    let r = s
        .execute(
            "infographic.inspectCsv",
            &json!({
                "csv": "Month,Sales,Profit\nJan,10,2\nFeb,20,5\n"
            }),
        )
        .unwrap();
    assert_eq!(r["rowCount"], 2);
    assert_eq!(r["columns"][0]["name"], "Month");
    assert_eq!(r["columns"][1]["inferredType"], "number");
}

#[test]
fn create_chart_adds_metadata_and_graph() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    let r = s
        .execute(
            "infographic.createChart",
            &json!({
                "csv": "Month,Sales\nJan,10\nFeb,20\nMar,15\n",
                "mapping": {"category":"Month","values":["Sales"]},
                "preset":"bar-building",
                "axis":{"y":"minimal","xLabelStyle":"bold","legend":false},
                "placement":{"x":100,"y":80,"width":500,"height":300}
            }),
        )
        .unwrap();
    let gid = id(&r);
    assert!(gid > 0);
    let listed = s.execute("infographic.listCharts", &json!({})).unwrap();
    assert_eq!(listed["charts"].as_array().unwrap().len(), 1);
    assert_eq!(listed["charts"][0]["preset"], "bar-building");
    assert_eq!(listed["charts"][0]["graphNodeId"], gid);
}

#[test]
fn add_multiple_charts_from_different_csv_sources() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1200, "height": 800})).unwrap();
    s.execute(
        "infographic.createChart",
        &json!({
            "csv":"Month,Sales\nJan,10\nFeb,20\n",
            "mapping":{"category":"Month","values":["Sales"]},
            "preset":"bar-building",
            "name":"Sales"
        }),
    )
    .unwrap();
    s.execute(
        "infographic.addChartFromCsv",
        &json!({
            "csv":"Month,Profit\nJan,2\nFeb,5\n",
            "mapping":{"category":"Month","values":["Profit"]},
            "preset":"bar-arrow-highlight",
            "highlight":{"mode":"max"},
            "name":"Profit"
        }),
    )
    .unwrap();
    let listed = s.execute("infographic.listCharts", &json!({})).unwrap();
    let charts = listed["charts"].as_array().unwrap();
    assert_eq!(charts.len(), 2);
    assert_eq!(charts[0]["name"], "Sales");
    assert_eq!(charts[1]["name"], "Profit");
    assert_eq!(charts[1]["source"]["path"], "inline");
}

#[test]
fn update_chart_is_single_regen_path_and_no_mutation_on_invalid() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    let created = s
        .execute(
            "infographic.createChart",
            &json!({
                "csv":"Month,Sales\nJan,10\nFeb,20\n",
                "mapping":{"category":"Month","values":["Sales"]},
                "preset":"bar-arrow-highlight",
                "highlight":{"mode":"last"}
            }),
        )
        .unwrap();
    let chart_id = created["chartId"].as_str().unwrap().to_string();
    let before = s.execute("infographic.listCharts", &json!({})).unwrap();

    let err = s
        .execute(
            "infographic.updateChart",
            &json!({
                "chartId": chart_id,
                "mapping":{"category":"Month","values":["Missing"]},
                "preset":"bar-arrow-highlight"
            }),
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("missing CSV column") || msg.contains("CSV"));

    let after = s.execute("infographic.listCharts", &json!({})).unwrap();
    assert_eq!(before["charts"], after["charts"]);

    let updated = s
        .execute(
            "infographic.updateChart",
            &json!({
                "chartId": chart_id,
                "csv":"Month,Sales\nJan,30\nFeb,40\nMar,50\n",
                "mapping":{"category":"Month","values":["Sales"]},
                "preset":"bar-arrow-highlight",
                "highlight":{"mode":"max"}
            }),
        )
        .unwrap();
    assert_eq!(updated["updated"], true);
    let listed = s.execute("infographic.listCharts", &json!({})).unwrap();
    assert_eq!(listed["charts"][0]["highlight"]["mode"], "max");

    // Update without re-supplying CSV (reuses stored source.csv) and verify overlay cleanup
    let first_overlays = updated["overlayNodeIds"].as_array().unwrap().clone();
    assert_eq!(first_overlays.len(), 1);
    let updated2 = s
        .execute(
            "infographic.updateChart",
            &json!({
                "chartId": chart_id,
                "highlight":{"mode":"min"}
            }),
        )
        .unwrap();
    let second_overlays = updated2["overlayNodeIds"].as_array().unwrap().clone();
    assert_eq!(second_overlays.len(), 1);
    assert_ne!(first_overlays[0], second_overlays[0]);

    // Remove chart cleans up graph + overlay + metadata
    let removed = s.execute("infographic.removeChart", &json!({"chartId": chart_id})).unwrap();
    assert_eq!(removed["removed"], true);
    let after_remove = s.execute("infographic.listCharts", &json!({})).unwrap();
    assert_eq!(after_remove["charts"].as_array().unwrap().len(), 0);
}

#[test]
fn list_and_place_decorations_with_tint() {
    let mut s = Session::new();
    let list = s.execute("infographic.listDecorations", &json!({})).unwrap();
    let items = list["decorations"].as_array().unwrap();
    assert!(items.len() >= 45);

    s.execute("file.new", &json!({"width": 800, "height": 600})).unwrap();
    let placed_doc_vec = s.execute("infographic.placeDecoration", &json!({"asset": "ikon-rumah-kecil", "at": [120, 120]})).unwrap();
    assert!(!placed_doc_vec["ids"].as_array().unwrap().is_empty());
    let placed = s.execute("infographic.placeDecoration", &json!({"asset": "trend-up", "at": [200, 150], "scale": 1.5, "tint": "#10B981"})).unwrap();
    assert_eq!(placed["recolored"], true);
    assert!(!placed["ids"].as_array().unwrap().is_empty());

    let err = s.execute("infographic.placeDecoration", &json!({"asset": "nonexistent-icon"})).unwrap_err();
    assert!(err.to_string().contains("unknown decoration"));
}

#[test]
fn create_from_csv_template_and_export() {
    let mut s = Session::new();
    let tpls = s.execute("infographic.listTemplates", &json!({})).unwrap();
    assert!(tpls["templates"].as_array().unwrap().len() >= 3);

    // Invalid template does not create a document
    let err = s
        .execute(
            "infographic.createFromCsv",
            &json!({
                "template": "unknown-template",
                "csv": "Month,Sales\nJan,10\n",
                "mapping": {"category": "Month", "values": ["Sales"]}
            }),
        )
        .unwrap_err();
    assert!(err.to_string().contains("unknown template"));
    assert!(s.doc().is_err());

    let created = s
        .execute(
            "infographic.createFromCsv",
            &json!({
                "template": "kpi-highlight-card",
                "newDocument": true,
                "csv": "Quarter,Revenue\nQ1,120\nQ2,180\nQ3,150\nQ4,240\n",
                "mapping": {"category": "Quarter", "values": ["Revenue"]},
                "title": "Annual Revenue",
                "subtitle": "FY 2026 Performance",
                "sourceNote": "Source: Internal Ledger"
            }),
        )
        .unwrap();
    assert_eq!(created["template"], "kpi-highlight-card");
    assert_eq!(created["textNodeIds"].as_array().unwrap().len(), 3);
    assert!(!created["decorationNodeIds"].as_array().unwrap().is_empty());

    let svg_path = std::env::temp_dir().join(format!("vc_infographic_test_{}.svg", std::process::id()));
    let _ = std::fs::remove_file(&svg_path);
    let exported = s.execute("infographic.export", &json!({"path": svg_path.to_string_lossy()})).unwrap();
    assert_eq!(exported["format"], "svg");
    assert!(std::fs::metadata(&svg_path).unwrap().len() > 0);
    let _ = std::fs::remove_file(&svg_path);
}

#[test]
fn new_vector_chart_presets_work() {
    let mut s = Session::new();
    s.execute("file.new", &json!({"width": 1200, "height": 800})).unwrap();
    for preset in ["bar-isometric-blue", "bar-stepped-ribbon", "bar-decline-green-blue", "bar-isometric-orange"] {
        let res = s
            .execute(
                "infographic.createChart",
                &json!({
                    "csv": "Month,Sales\nJan,10\nFeb,25\nMar,18\n",
                    "mapping": {"category": "Month", "values": ["Sales"]},
                    "preset": preset,
                }),
            )
            .unwrap();
        assert_eq!(res["preset"], preset);
        assert!(res["graphNodeId"].as_u64().unwrap() > 0);
    }
}
