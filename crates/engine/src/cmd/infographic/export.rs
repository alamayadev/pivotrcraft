use serde_json::{Value, json};

use super::{bad_param, text_or};

const C_EXPORT: &str = "infographic.export";

pub fn export_infographic(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let path = text_or(p, "path").ok_or_else(|| bad_param(C_EXPORT, "missing `path`"))?;
    let format = text_or(p, "format").unwrap_or_else(|| infer_format(path));
    if !matches!(format, "png" | "svg" | "svgz" | "pdf" | "jpg" | "jpeg" | "webp") {
        return Err(bad_param(C_EXPORT, format!("unsupported export format `{format}`")));
    }

    let mut export_params = json!({
        "path": path,
        "format": format,
    });
    if let Some(scale) = p.get("scale") {
        export_params["scale"] = scale.clone();
    }
    if let Some(use_artboards) = p.get("useArtboards") {
        export_params["useArtboards"] = use_artboards.clone();
    }

    let res = s.execute("document.export", &export_params)?;
    Ok(json!({
        "path": res.get("path").cloned().unwrap_or_else(|| json!(path)),
        "format": format,
        "warnings": res.get("warnings").cloned().unwrap_or_else(|| json!([])),
    }))
}

fn infer_format(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".svgz") {
        "svgz"
    } else if lower.ends_with(".svg") {
        "svg"
    } else if lower.ends_with(".pdf") {
        "pdf"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "jpg"
    } else if lower.ends_with(".webp") {
        "webp"
    } else {
        "png"
    }
}
