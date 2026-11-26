use serde_json::{Value, json};

pub fn list_presets() -> Vec<Value> {
    vec![
        json!({
            "id": "bar-building",
            "label": "Building Bars",
            "kind": "bar",
            "description": "Sliding column bar preset with fixed building silhouette art and limited palette variants.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none"],
            "paletteVariants": ["blue-2tone"]
        }),
        json!({
            "id": "bar-arrow-highlight",
            "label": "Bar Arrow Highlight",
            "kind": "bar",
            "description": "Regular bar chart with conditional arrow-cap highlight overlay.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none","last","max","min","index"],
            "paletteVariants": ["default"]
        }),
        json!({
            "id": "bar-isometric-blue",
            "label": "Isometric Blue Growth",
            "kind": "bar",
            "description": "Isometric 3D blue column preset inspired by docs/vector/isometric-blue-growth-chart.svg.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none"],
            "paletteVariants": ["isometric-blue"]
        }),
        json!({
            "id": "bar-stepped-ribbon",
            "label": "Stepped Ribbon Bars",
            "kind": "bar",
            "description": "Stepped ribbon column preset inspired by docs/vector/stepped-ribbon-upward-arrow-chart.svg.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none"],
            "paletteVariants": ["terracotta-slate"]
        }),
        json!({
            "id": "bar-decline-green-blue",
            "label": "Green-Blue Contrast",
            "kind": "bar",
            "description": "Teal-to-blue beveled column preset inspired by docs/vector/green-blue-decline-chart.svg.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none"],
            "paletteVariants": ["green-blue"]
        }),
        json!({
            "id": "bar-isometric-orange",
            "label": "Isometric Orange Rising",
            "kind": "bar",
            "description": "Isometric orange arrow-bar column preset inspired by docs/vector/isometric-orange-rising-arrows-chart.svg.",
            "axisDefaults": {"y":"minimal","xLabelStyle":"bold","legend":false},
            "highlightModes": ["none"],
            "paletteVariants": ["isometric-orange"]
        }),
    ]
}

pub fn is_supported_preset(id: &str) -> bool {
    matches!(
        id,
        "bar-building" | "bar-arrow-highlight" | "bar-isometric-blue" | "bar-stepped-ribbon" | "bar-decline-green-blue" | "bar-isometric-orange"
    )
}

pub fn default_preset() -> &'static str {
    "bar-building"
}
