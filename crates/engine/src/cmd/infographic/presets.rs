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
    ]
}

pub fn is_supported_preset(id: &str) -> bool {
    matches!(id, "bar-building" | "bar-arrow-highlight")
}

pub fn default_preset() -> &'static str {
    "bar-building"
}
