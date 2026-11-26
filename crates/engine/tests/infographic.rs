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
}
