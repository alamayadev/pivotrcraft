pub mod chart;
pub mod csv;
pub mod metadata;
pub mod presets;

use serde_json::{Value, json};

use super::{CommandSpec, Result, Session, always, bad, cmd, has_doc, str_param};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            query "infographic.inspectCsv",
            "Inspect CSV",
            [],
            None,
            "{path?|data?|dataBase64?|csv?, delimiter?: auto|comma|tab|semicolon, maxRows?: 1-200 (default 20)} inspect CSV data for infographic mapping -> {columns:[{name,index,inferredType,sampleValues}], rowCount, sampleRows, warnings}",
            always,
            inspect_csv
        ),
        cmd!(
            query "infographic.listChartPresets",
            "List Chart Presets",
            [],
            None,
            "{} list built-in infographic chart presets -> {presets:[{id,label,kind,axisDefaults,highlightModes,description}]}",
            always,
            list_chart_presets
        ),
        cmd!(
            query "infographic.listCharts",
            "List Charts",
            [],
            None,
            "{infographicId?} list infographic chart blocks in the active document -> {charts:[...], warnings:[...]}",
            has_doc,
            list_charts
        ),
        cmd!(
            "infographic.createChart",
            "Create Chart",
            [],
            None,
            "{csvPath?|csvData?|csvBase64?|csv?, mapping:{category,values:[...],series?,missing?:skip|zero|error,aggregation?:none|sum|avg|count|min|max}, preset?:bar-building|bar-arrow-highlight, axis?:{y?:minimal|hidden,xLabelStyle?:bold|normal,legend?:bool}, highlight?:{mode?:none|last|max|min|index,index?}, placement?:{x?,y?,width?,height?,artboardId?}, name?} create an editable infographic chart block from CSV data",
            has_doc,
            create_chart
        ),
        cmd!(
            "infographic.addChartFromCsv",
            "Add Chart from CSV",
            [],
            None,
            "{infographicId?, ...same as infographic.createChart} add another chart block to an infographic page/artboard",
            has_doc,
            add_chart_from_csv
        ),
        cmd!(
            "infographic.updateChart",
            "Update Chart",
            [],
            None,
            "{chartId, csvPath?|csvData?|csvBase64?|csv?, mapping?, preset?, axis?, highlight?, name?} deterministic regen path for a chart block; cleans up stale generated overlays",
            has_doc,
            update_chart
        ),
    ]
}

fn inspect_csv(_s: &mut Session, p: &Value) -> Result<Value> {
    crate::cmd::infographic::csv::inspect_csv(p)
}

fn list_chart_presets(_s: &mut Session, _p: &Value) -> Result<Value> {
    Ok(json!({ "presets": crate::cmd::infographic::presets::list_presets() }))
}

fn list_charts(s: &mut Session, p: &Value) -> Result<Value> {
    crate::cmd::infographic::chart::list_charts(s, p)
}

fn create_chart(s: &mut Session, p: &Value) -> Result<Value> {
    crate::cmd::infographic::chart::create_chart(s, p)
}

fn add_chart_from_csv(s: &mut Session, p: &Value) -> Result<Value> {
    crate::cmd::infographic::chart::add_chart_from_csv(s, p)
}

fn update_chart(s: &mut Session, p: &Value) -> Result<Value> {
    crate::cmd::infographic::chart::update_chart(s, p)
}

pub(crate) fn bad_param(cmd_name: &str, msg: impl Into<String>) -> crate::EngineError {
    bad(cmd_name, msg)
}

pub(crate) fn text_or<'a>(p: &'a Value, key: &str) -> Option<&'a str> {
    str_param(p, key).map(str::trim).filter(|s| !s.is_empty())
}
