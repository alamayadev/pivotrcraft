use std::sync::Arc;

use serde_json::{Value, json};
use vectorcraft_color::{Color, Paint};
use vectorcraft_doc::{Node, NodeId};

use super::metadata::{ensure_infographic, read_doc_meta, write_doc_meta};
use super::{bad_param, text_or};

const C_PLACE: &str = "infographic.placeDecoration";

#[derive(Clone, Copy)]
pub struct DecorationSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub tags: &'static [&'static str],
    pub recolorable: bool,
    pub default_width: f64,
    pub default_height: f64,
    pub svg: &'static str,
}

const DECORATIONS: &[DecorationSpec] = &[
    DecorationSpec {
        id: "trend-up",
        label: "Trend Up Badge",
        category: "indicators",
        tags: &["trend", "growth", "up", "arrow"],
        recolorable: true,
        default_width: 64.0,
        default_height: 64.0,
        svg: include_str!("../../../../../assets/infographic/decorations/trend-up.svg"),
    },
    DecorationSpec {
        id: "trend-down",
        label: "Trend Down Badge",
        category: "indicators",
        tags: &["trend", "decline", "down", "arrow"],
        recolorable: true,
        default_width: 64.0,
        default_height: 64.0,
        svg: include_str!("../../../../../assets/infographic/decorations/trend-down.svg"),
    },
    DecorationSpec {
        id: "target-ring",
        label: "Target Ring",
        category: "icons",
        tags: &["target", "goal", "kpi", "ring"],
        recolorable: true,
        default_width: 64.0,
        default_height: 64.0,
        svg: include_str!("../../../../../assets/infographic/decorations/target-ring.svg"),
    },
    DecorationSpec {
        id: "accent-dots",
        label: "Dot Matrix Accent",
        category: "accents",
        tags: &["dots", "pattern", "background", "accent"],
        recolorable: true,
        default_width: 80.0,
        default_height: 60.0,
        svg: include_str!("../../../../../assets/infographic/decorations/accent-dots.svg"),
    },
    DecorationSpec {
        id: "callout-ribbon",
        label: "Callout Ribbon",
        category: "banners",
        tags: &["ribbon", "banner", "header", "callout"],
        recolorable: false,
        default_width: 240.0,
        default_height: 48.0,
        svg: include_str!("../../../../../assets/infographic/decorations/callout-ribbon.svg"),
    },
    DecorationSpec {
        id: "spark-star",
        label: "Spark Star",
        category: "accents",
        tags: &["star", "spark", "highlight", "accent"],
        recolorable: true,
        default_width: 64.0,
        default_height: 64.0,
        svg: include_str!("../../../../../assets/infographic/decorations/spark-star.svg"),
    },
    // Additional vector objects from docs/vector/
    DecorationSpec {
        id: "chart-donut-belanja",
        label: "Donut Chart (Spending)",
        category: "charts",
        tags: &["chart", "donut", "spending"],
        recolorable: false,
        default_width: 160.0,
        default_height: 160.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/chart-donut-belanja.svg"),
    },
    DecorationSpec {
        id: "chart-gauge-pendidikan",
        label: "Gauge Chart (Education)",
        category: "charts",
        tags: &["chart", "gauge", "education"],
        recolorable: false,
        default_width: 160.0,
        default_height: 120.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/chart-gauge-pendidikan.svg"),
    },
    DecorationSpec {
        id: "chart-pns-gender",
        label: "Gender Comparison Chart",
        category: "charts",
        tags: &["chart", "gender", "people"],
        recolorable: false,
        default_width: 200.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/chart-pns-gender.svg"),
    },
    DecorationSpec {
        id: "ikon-dompet-uang",
        label: "Wallet Icon",
        category: "finance",
        tags: &["wallet", "money", "finance"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/ikon-dompet-uang.svg"),
    },
    DecorationSpec {
        id: "ikon-koin-emas",
        label: "Gold Coins Icon",
        category: "finance",
        tags: &["coins", "gold", "finance"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/ikon-koin-emas.svg"),
    },
    DecorationSpec {
        id: "ikon-timbangan-neraca",
        label: "Balance Scale Icon",
        category: "icons",
        tags: &["scale", "balance", "justice"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/ikon-timbangan-neraca.svg"),
    },
    DecorationSpec {
        id: "ilustrasi-buku-wisuda",
        label: "Education Book & Cap",
        category: "illustrations",
        tags: &["book", "graduation", "education"],
        recolorable: false,
        default_width: 160.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/ilustrasi-buku-wisuda.svg"),
    },
    DecorationSpec {
        id: "ornamen-geometris-kanan",
        label: "Geometric Corner Ornament",
        category: "accents",
        tags: &["ornament", "geometric", "corner"],
        recolorable: false,
        default_width: 140.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan2_svg/ornamen-geometris-kanan.svg"),
    },
    DecorationSpec {
        id: "chart-bar-kecamatan",
        label: "District Bar Chart",
        category: "charts",
        tags: &["chart", "bar", "district"],
        recolorable: false,
        default_width: 180.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan3_svg/chart-bar-kecamatan.svg"),
    },
    DecorationSpec {
        id: "chart-pie-luas-wilayah",
        label: "Area Pie Chart",
        category: "charts",
        tags: &["chart", "pie", "area"],
        recolorable: false,
        default_width: 160.0,
        default_height: 160.0,
        svg: include_str!("../../../../../docs/vector/bahan3_svg/chart-pie-luas-wilayah.svg"),
    },
    DecorationSpec {
        id: "ikon-lingkaran-desa",
        label: "Village Circle Badge",
        category: "icons",
        tags: &["village", "badge", "circle"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan3_svg/ikon-lingkaran-desa.svg"),
    },
    DecorationSpec {
        id: "ikon-termometer-suhu",
        label: "Thermometer Icon",
        category: "environment",
        tags: &["thermometer", "temperature", "weather"],
        recolorable: false,
        default_width: 80.0,
        default_height: 180.0,
        svg: include_str!("../../../../../docs/vector/bahan3_svg/ikon-termometer-suhu.svg"),
    },
    DecorationSpec {
        id: "ikon-tetes-hujan",
        label: "Raindrop Icon",
        category: "environment",
        tags: &["rain", "water", "weather"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan3_svg/ikon-tetes-hujan.svg"),
    },
    DecorationSpec {
        id: "ikon-bank-pemerintah",
        label: "Bank Building Icon",
        category: "finance",
        tags: &["bank", "building", "finance"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-bank-pemerintah.svg"),
    },
    DecorationSpec {
        id: "ikon-bpr-koin",
        label: "Microfinance Coin Icon",
        category: "finance",
        tags: &["bank", "coin", "finance"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-bpr-koin.svg"),
    },
    DecorationSpec {
        id: "ikon-kud-roda-gigi",
        label: "Cooperative Gear Icon",
        category: "icons",
        tags: &["gear", "cooperative", "industry"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-kud-roda-gigi.svg"),
    },
    DecorationSpec {
        id: "ikon-pasar-tradisional",
        label: "Traditional Market Icon",
        category: "commerce",
        tags: &["market", "shop", "commerce"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-pasar-tradisional.svg"),
    },
    DecorationSpec {
        id: "ikon-rak-minimarket",
        label: "Retail Shelf Icon",
        category: "commerce",
        tags: &["retail", "shelf", "commerce"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-rak-minimarket.svg"),
    },
    DecorationSpec {
        id: "ikon-restoran-piring",
        label: "Restaurant Plate Icon",
        category: "commerce",
        tags: &["restaurant", "food", "plate"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-restoran-piring.svg"),
    },
    DecorationSpec {
        id: "ikon-rumah-kecil",
        label: "Small House Icon",
        category: "buildings",
        tags: &["house", "home", "building"],
        recolorable: false,
        default_width: 100.0,
        default_height: 90.0,
        svg: include_str!("../../../../../docs/vector/bahan4_svg/ikon-rumah-kecil.svg"),
    },
    DecorationSpec {
        id: "ikon-ekspedisi-swasta",
        label: "Courier Service Icon",
        category: "logistics",
        tags: &["courier", "delivery", "logistics"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan5_svg/ikon-ekspedisi-swasta.svg"),
    },
    DecorationSpec {
        id: "ikon-layanan-pos",
        label: "Postal Service Icon",
        category: "logistics",
        tags: &["post", "mail", "logistics"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan5_svg/ikon-layanan-pos.svg"),
    },
    DecorationSpec {
        id: "ikon-nihil-penginapan",
        label: "No Lodging Icon",
        category: "buildings",
        tags: &["hotel", "lodging", "none"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan5_svg/ikon-nihil-penginapan.svg"),
    },
    DecorationSpec {
        id: "objek-gedung-hotel",
        label: "Hotel Building Object",
        category: "buildings",
        tags: &["hotel", "building", "isometric"],
        recolorable: false,
        default_width: 160.0,
        default_height: 160.0,
        svg: include_str!("../../../../../docs/vector/bahan5_svg/objek-gedung-hotel.svg"),
    },
    DecorationSpec {
        id: "objek-jaringan-transportasi",
        label: "Transport Network Object",
        category: "logistics",
        tags: &["transport", "network", "road"],
        recolorable: false,
        default_width: 180.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan5_svg/objek-jaringan-transportasi.svg"),
    },
    DecorationSpec {
        id: "ikon-aksesibilitas-lingkaran",
        label: "Accessibility Ring Icon",
        category: "health",
        tags: &["accessibility", "ring", "health"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-aksesibilitas-lingkaran.svg"),
    },
    DecorationSpec {
        id: "ikon-kubus-apotek",
        label: "Pharmacy Cube Icon",
        category: "health",
        tags: &["pharmacy", "cube", "health"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-kubus-apotek.svg"),
    },
    DecorationSpec {
        id: "ikon-kubus-puskesmas",
        label: "Clinic Cube Icon",
        category: "health",
        tags: &["clinic", "cube", "health"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-kubus-puskesmas.svg"),
    },
    DecorationSpec {
        id: "ikon-kubus-rumah-sakit",
        label: "Hospital Cube Icon",
        category: "health",
        tags: &["hospital", "cube", "health"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-kubus-rumah-sakit.svg"),
    },
    DecorationSpec {
        id: "ikon-menara-listrik",
        label: "Power Tower Icon",
        category: "infrastructure",
        tags: &["power", "electricity", "tower"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-menara-listrik.svg"),
    },
    DecorationSpec {
        id: "ikon-peringatan-segitiga",
        label: "Warning Triangle Icon",
        category: "safety",
        tags: &["warning", "triangle", "alert"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-peringatan-segitiga.svg"),
    },
    DecorationSpec {
        id: "ikon-rompi-keselamatan",
        label: "Safety Vest Icon",
        category: "safety",
        tags: &["safety", "vest", "gear"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-rompi-keselamatan.svg"),
    },
    DecorationSpec {
        id: "ikon-sirine-peringatan",
        label: "Warning Siren Icon",
        category: "safety",
        tags: &["siren", "alert", "safety"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-sirine-peringatan.svg"),
    },
    DecorationSpec {
        id: "ikon-tabung-gas",
        label: "Gas Cylinder Icon",
        category: "infrastructure",
        tags: &["gas", "cylinder", "energy"],
        recolorable: false,
        default_width: 100.0,
        default_height: 100.0,
        svg: include_str!("../../../../../docs/vector/bahan6_svg/ikon-tabung-gas.svg"),
    },
    DecorationSpec {
        id: "bingkai-geometris-bps",
        label: "Geometric Frame",
        category: "accents",
        tags: &["frame", "geometric", "border"],
        recolorable: false,
        default_width: 240.0,
        default_height: 160.0,
        svg: include_str!("../../../../../docs/vector/bahan7_svg/bingkai-geometris-bps.svg"),
    },
    DecorationSpec {
        id: "ilustrasi-tanaman-hias",
        label: "Ornamental Plant Illustration",
        category: "illustrations",
        tags: &["plant", "botanical", "illustration"],
        recolorable: false,
        default_width: 160.0,
        default_height: 160.0,
        svg: include_str!("../../../../../docs/vector/bahan7_svg/ilustrasi-tanaman-hias.svg"),
    },
    DecorationSpec {
        id: "chart-tabel-perbandingan",
        label: "Comparison Table Chart",
        category: "charts",
        tags: &["chart", "table", "comparison"],
        recolorable: false,
        default_width: 200.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan8_svg/chart-tabel-perbandingan.svg"),
    },
    DecorationSpec {
        id: "chart-timeline-jarak",
        label: "Distance Timeline Chart",
        category: "charts",
        tags: &["chart", "timeline", "distance"],
        recolorable: false,
        default_width: 220.0,
        default_height: 120.0,
        svg: include_str!("../../../../../docs/vector/bahan8_svg/chart-timeline-jarak.svg"),
    },
    DecorationSpec {
        id: "chart-treemap-luas-wilayah",
        label: "Area Treemap Chart",
        category: "charts",
        tags: &["chart", "treemap", "area"],
        recolorable: false,
        default_width: 200.0,
        default_height: 140.0,
        svg: include_str!("../../../../../docs/vector/bahan8_svg/chart-treemap-luas-wilayah.svg"),
    },
];

pub fn find_decoration(id: &str) -> Option<&'static DecorationSpec> {
    DECORATIONS.iter().find(|d| d.id == id)
}

pub fn list_decorations(p: &Value) -> Value {
    let cat = text_or(p, "category");
    let tag = text_or(p, "tag");
    let items: Vec<Value> = DECORATIONS
        .iter()
        .filter(|d| cat.is_none_or(|c| d.category.eq_ignore_ascii_case(c)))
        .filter(|d| tag.is_none_or(|t| d.tags.iter().any(|dt| dt.eq_ignore_ascii_case(t))))
        .map(|d| {
            json!({
                "id": d.id,
                "label": d.label,
                "category": d.category,
                "tags": d.tags,
                "recolorable": d.recolorable,
                "defaultWidth": d.default_width,
                "defaultHeight": d.default_height,
            })
        })
        .collect();
    json!({ "decorations": items })
}

pub fn place_decoration(s: &mut crate::Session, p: &Value) -> crate::Result<Value> {
    let id = text_or(p, "asset").or_else(|| text_or(p, "id")).ok_or_else(|| bad_param(C_PLACE, "missing `asset`"))?;
    let spec = find_decoration(id).ok_or_else(|| bad_param(C_PLACE, format!("unknown decoration `{id}`")))?;

    let tint = text_or(p, "tint").map(parse_hex_color).transpose()?;
    let scale = match p.get("scale").and_then(Value::as_f64) {
        Some(sc) if !sc.is_finite() || sc <= 0.0 => return Err(bad_param(C_PLACE, "`scale` must be finite and > 0")),
        Some(sc) => sc,
        None => 1.0,
    };

    let mut place_params = json!({
        "name": format!("{}.svg", spec.id),
        "dataBase64": vectorcraft_format::base64_encode(spec.svg.as_bytes()),
    });
    if let Some(at) = p.get("at") {
        place_params["at"] = at.clone();
    }
    if let Some(rect) = p.get("rect") {
        place_params["rect"] = rect.clone();
    } else if (scale - 1.0).abs() > f64::EPSILON {
        let (cx, cy) = match p.get("at").and_then(Value::as_array) {
            Some(a) if a.len() == 2 => (a[0].as_f64().unwrap_or(200.0), a[1].as_f64().unwrap_or(200.0)),
            _ => (200.0, 200.0),
        };
        let w = spec.default_width * scale;
        let h = spec.default_height * scale;
        place_params["rect"] = json!([cx - w * 0.5, cy - h * 0.5, w, h]);
    }

    let placed = s.execute("file.place", &place_params)?;
    let placed_ids: Vec<u64> = placed.get("ids").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();

    let mut warnings: Vec<String> = Vec::new();
    if let Some(color) = tint {
        if spec.recolorable {
            let ids = placed_ids.clone();
            s.edit("Tint Decoration", |doc, _sel| {
                for pid in ids {
                    if let Some(n) = doc.node_mut(NodeId(pid)) {
                        tint_node_solid_paints(n, color);
                    }
                }
                Ok(())
            })?;
        } else {
            warnings.push(format!("decoration `{}` preserves multi-color art; `tint` ignored", spec.id));
        }
    }

    let infographic_id = text_or(p, "infographicId");
    let st = s.doc_mut()?;
    let (mut root, _) = read_doc_meta(&st.doc);
    if let Some(info) = ensure_infographic(&mut root, infographic_id, None) {
        let objects = info.entry("objects").or_insert_with(|| json!([]));
        if !objects.is_array() {
            *objects = json!([]);
        }
        if let Some(arr) = objects.as_array_mut() {
            let obj_id = format!("obj-{}", arr.len() + 1);
            arr.push(json!({
                "id": obj_id,
                "asset": spec.id,
                "nodeIds": placed_ids,
                "nodeId": placed_ids.first().copied(),
            }));
        }
        write_doc_meta(Arc::make_mut(&mut st.doc), root);
    }

    Ok(json!({
        "asset": spec.id,
        "ids": placed_ids,
        "recolored": tint.is_some() && spec.recolorable,
        "warnings": warnings,
    }))
}

fn tint_node_solid_paints(node: &mut Node, color: Color) {
    let mut app = (*node.appearance).clone();
    for item in &mut app.items {
        let paint = match item {
            vectorcraft_doc::AppearanceItem::Fill(f) => &mut f.paint,
            vectorcraft_doc::AppearanceItem::Stroke(st) => &mut st.paint,
        };
        if let Paint::Solid { color: c, .. } = paint {
            *c = color;
        }
    }
    node.appearance = vectorcraft_doc::Shared::new(app);
    if let Some(children) = node.children_mut() {
        for c in children.iter_mut() {
            tint_node_solid_paints(Arc::make_mut(c), color);
        }
    }
}

fn parse_hex_color(raw: &str) -> crate::Result<Color> {
    let s = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    let parse_byte = |h: &str| u8::from_str_radix(h, 16).map_err(|_| bad_param(C_PLACE, format!("invalid hex color `{raw}`")));
    match s.len() {
        6 => {
            let r = parse_byte(&s[0..2])? as f32 / 255.0;
            let g = parse_byte(&s[2..4])? as f32 / 255.0;
            let b = parse_byte(&s[4..6])? as f32 / 255.0;
            Ok(Color::rgb(r, g, b))
        }
        3 => {
            let r = parse_byte(&s[0..1])? * 17;
            let g = parse_byte(&s[1..2])? * 17;
            let b = parse_byte(&s[2..3])? * 17;
            Ok(Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
        }
        _ => Err(bad_param(C_PLACE, format!("invalid hex color `{raw}` (expected #RRGGBB or #RGB)"))),
    }
}
