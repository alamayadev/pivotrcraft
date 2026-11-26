use serde_json::{Value, json};

use super::{bad_param, text_or};

#[derive(Clone, Debug)]
pub struct CsvInput {
    pub bytes: Vec<u8>,
    pub source: String,
}

pub fn inspect_csv(p: &Value) -> crate::Result<Value> {
    const C: &str = "infographic.inspectCsv";
    let input = load_csv_input(p, C)?;
    let delim = delimiter_from(p)?;
    let max_rows = p.get("maxRows").and_then(Value::as_u64).unwrap_or(20).clamp(1, 200) as usize;
    let text = decode_csv_bytes(&input.bytes, C)?;
    let rows = parse_rows(&text, delim);
    let mut warnings: Vec<String> = Vec::new();
    if input.bytes.len() > 1_048_576 {
        return Err(bad_param(C, "CSV is larger than 1048576 bytes"));
    }
    if rows.is_empty() {
        return Err(bad_param(C, "CSV has no rows"));
    }
    let header = rows[0].clone();
    let mut columns = Vec::new();
    for (i, name) in header.iter().enumerate() {
        let mut samples = Vec::new();
        let mut numeric = 0usize;
        let mut nonempty = 0usize;
        for row in rows.iter().skip(1).take(max_rows) {
            let cell = row.get(i).cloned().unwrap_or_default();
            let trimmed = cell.trim();
            if !trimmed.is_empty() {
                nonempty += 1;
                if parse_number(trimmed).is_some() {
                    numeric += 1;
                }
                if samples.len() < 5 {
                    samples.push(Value::String(cell.clone()));
                }
            }
        }
        let inferred_type = if nonempty == 0 {
            "empty"
        } else if numeric == nonempty {
            "number"
        } else if numeric > 0 {
            "mixed"
        } else {
            "string"
        };
        columns.push(json!({
            "name": if name.trim().is_empty() { format!("column{}", i + 1) } else { name.clone() },
            "index": i,
            "inferredType": inferred_type,
            "sampleValues": samples,
        }));
    }
    let sample_rows: Vec<Value> = rows.iter().skip(1).take(max_rows).map(|r| json!(r)).collect();
    let row_count = rows.len().saturating_sub(1);
    if row_count == 0 {
        warnings.push("CSV has header but no data rows".into());
    }
    Ok(json!({
        "source": input.source,
        "delimiter": match delim { Delimiter::Comma => "comma", Delimiter::Tab => "tab", Delimiter::Semicolon => "semicolon", Delimiter::Auto => "auto" },
        "columns": columns,
        "rowCount": row_count,
        "sampleRows": sample_rows,
        "warnings": warnings,
    }))
}

pub fn load_csv_input(p: &Value, cmd: &str) -> crate::Result<CsvInput> {
    if let Some(path) = text_or(p, "csvPath").or_else(|| text_or(p, "path")) {
        let bytes = crate::cmd::fileio::read_file_head(path, 1_048_576 + 1)?;
        if bytes.len() > 1_048_576 {
            return Err(bad_param(cmd, "CSV is larger than 1048576 bytes"));
        }
        return Ok(CsvInput { bytes, source: path.to_string() });
    }
    if let Some(data) = text_or(p, "csvData").or_else(|| text_or(p, "data")).or_else(|| text_or(p, "csv")) {
        return Ok(CsvInput { bytes: data.as_bytes().to_vec(), source: "inline".into() });
    }
    if let Some(b64) = text_or(p, "csvBase64").or_else(|| text_or(p, "dataBase64")) {
        let bytes = vectorcraft_format::base64_decode(b64).ok_or_else(|| bad_param(cmd, "bad csvBase64"))?;
        return Ok(CsvInput { bytes, source: "base64".into() });
    }
    Err(bad_param(cmd, "give `csvPath`, `csvData`, `csvBase64`, or `csv`"))
}

pub fn decode_csv_bytes(bytes: &[u8], cmd: &str) -> crate::Result<String> {
    if bytes.len() > 1_048_576 {
        return Err(bad_param(cmd, "CSV is larger than 1048576 bytes"));
    }
    match bytes {
        [0xFF, 0xFE, rest @ ..] => decode_utf16(rest, true).ok_or_else(|| bad_param(cmd, "CSV is not valid UTF-16LE")),
        [0xFE, 0xFF, rest @ ..] => decode_utf16(rest, false).ok_or_else(|| bad_param(cmd, "CSV is not valid UTF-16BE")),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8(rest.to_vec()).map_err(|_| bad_param(cmd, "CSV is not valid UTF-8")),
        _ => {
            if bytes.contains(&0) {
                if let Some(le) = crate::cmd::variablecmds::utf16_order_without_mark(bytes) {
                    return decode_utf16(bytes, le).ok_or_else(|| bad_param(cmd, "CSV is not valid UTF-16"));
                }
                return Err(bad_param(cmd, "CSV appears binary/NUL-containing and is not valid UTF-16"));
            }
            String::from_utf8(bytes.to_vec()).or_else(|_| Ok(bytes.iter().map(|&b| crate::cmd::place::text::windows_1252(b)).collect()))
        }
    }
}

fn decode_utf16(bytes: &[u8], le: bool) -> Option<String> {
    let units: Vec<u16> =
        bytes.chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
    String::from_utf16(&units).ok()
}

#[derive(Clone, Copy, Debug)]
pub enum Delimiter {
    Auto,
    Comma,
    Tab,
    Semicolon,
}

fn delimiter_from(p: &Value) -> crate::Result<Delimiter> {
    match text_or(p, "delimiter") {
        None | Some("auto") => Ok(Delimiter::Auto),
        Some("comma") => Ok(Delimiter::Comma),
        Some("tab") => Ok(Delimiter::Tab),
        Some("semicolon") => Ok(Delimiter::Semicolon),
        Some(other) => Err(bad_param("infographic.inspectCsv", format!("unknown delimiter `{other}`"))),
    }
}

pub fn parse_rows(text: &str, delim: Delimiter) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let sep = match delim {
        Delimiter::Comma => ',',
        Delimiter::Tab => '\t',
        Delimiter::Semicolon => ';',
        Delimiter::Auto => detect_delimiter(text),
    };
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if in_quotes {
            if ch == '"' {
                if chars.get(i + 1) == Some(&'"') {
                    cell.push('"');
                    i += 2;
                    continue;
                }
                in_quotes = false;
                i += 1;
                continue;
            }
            cell.push(ch);
            i += 1;
            continue;
        }
        match ch {
            '"' => {
                in_quotes = true;
                i += 1;
            }
            c if c == sep => {
                row.push(cell.clone());
                cell.clear();
                i += 1;
            }
            '\n' => {
                row.push(cell.clone());
                cell.clear();
                if !(row.len() == 1 && row[0].trim().is_empty()) {
                    rows.push(row.clone());
                }
                row.clear();
                i += 1;
            }
            '\r' => {
                i += 1;
            }
            _ => {
                cell.push(ch);
                i += 1;
            }
        }
    }
    row.push(cell);
    if !(row.len() == 1 && row[0].trim().is_empty()) {
        rows.push(row);
    }
    rows
}

fn detect_delimiter(text: &str) -> char {
    let first = text.lines().next().unwrap_or("");
    let mut counts = [(',', 0usize), ('\t', 0usize), (';', 0usize)];
    let mut in_quotes = false;
    for ch in first.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        for (d, n) in counts.iter_mut() {
            if *d == ch {
                *n += 1;
            }
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).filter(|(_, n)| *n > 0).map(|(d, _)| d).unwrap_or(',')
}

pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let t = t.replace(',', "");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}
