// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct FileData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    row_count: usize,
    file_name: String,
    file_type: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

/// Parse CSV file and return structured data
#[tauri::command]
fn parse_csv(file_path: String) -> Result<FileData, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());
    
    let headers = reader.headers()
        .map_err(|e| format!("Failed to read headers: {}", e))?
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read record: {}", e))?;
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        rows.push(row);
    }
    
    let row_count = rows.len();
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    Ok(FileData {
        headers,
        rows,
        row_count,
        file_name,
        file_type: "CSV".to_string(),
    })
}

/// Flatten a JSON object into dot-notation keys and string values.
/// e.g. {"user": {"name": "Alice", "age": 28}} -> [("user.name", "Alice"), ("user.age", "28")]
fn flatten_object(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_object(&key, v, out);
            }
        }
        serde_json::Value::String(s) => out.push((prefix.to_string(), s.clone())),
        serde_json::Value::Number(n) => out.push((prefix.to_string(), n.to_string())),
        serde_json::Value::Bool(b) => out.push((prefix.to_string(), b.to_string())),
        serde_json::Value::Null => out.push((prefix.to_string(), String::new())),
        serde_json::Value::Array(arr) => {
            // Short arrays of primitives get joined, otherwise show as JSON
            let all_primitive = arr.iter().all(|v| !v.is_object() && !v.is_array());
            if all_primitive && arr.len() <= 10 {
                let items: Vec<String> = arr.iter().map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                }).collect();
                out.push((prefix.to_string(), items.join(", ")));
            } else {
                out.push((prefix.to_string(), value.to_string()));
            }
        }
    }
}

/// Extract the data array from a JSON value:
/// - Already an array of objects -> use directly
/// - A dict of objects (each key maps to an object) -> each key becomes a row with a "Name" column
/// - An object with a key whose value is the largest array of objects -> use that array
/// - A single object -> wrap in a one-element array
fn extract_data_array(parsed: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    match parsed {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return Err("JSON array is empty".to_string());
            }
            Ok(arr)
        }
        serde_json::Value::Object(map) => {
            // Check for dictionary-of-objects pattern: {"key1": {...}, "key2": {...}}
            let obj_value_count = map.values().filter(|v| v.is_object()).count();
            if obj_value_count > 1 && obj_value_count * 2 >= map.len() {
                let mut rows = Vec::new();
                for (key, value) in &map {
                    if let serde_json::Value::Object(inner) = value {
                        let mut row = serde_json::Map::new();
                        row.insert("Name".to_string(), serde_json::Value::String(key.clone()));
                        for (k, v) in inner {
                            row.insert(k.clone(), v.clone());
                        }
                        rows.push(serde_json::Value::Object(row));
                    }
                }
                return Ok(rows);
            }

            // Find the key with the largest array-of-objects value
            let best_key = map.iter()
                .filter_map(|(k, v)| {
                    if let serde_json::Value::Array(arr) = v {
                        if !arr.is_empty() && arr[0].is_object() {
                            return Some((k.clone(), arr.len()));
                        }
                    }
                    None
                })
                .max_by_key(|(_, len)| *len)
                .map(|(k, _)| k);

            if let Some(key) = best_key {
                if let Some(serde_json::Value::Array(arr)) = map.get(&key) {
                    return Ok(arr.clone());
                }
            }

            // No nested array found — treat the object itself as a single row
            Ok(vec![serde_json::Value::Object(map)])
        }
        _ => Err("JSON must be an object or an array of objects".to_string()),
    }
}

/// Parse JSON file and return structured data
#[tauri::command]
fn parse_json(file_path: String) -> Result<FileData, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let data = extract_data_array(parsed)?;

    // Flatten all rows and collect every header we see (preserving order of first appearance)
    let mut all_flat: Vec<Vec<(String, String)>> = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    let mut header_set = std::collections::HashSet::new();

    for item in &data {
        let mut pairs = Vec::new();
        flatten_object("", item, &mut pairs);
        for (key, _) in &pairs {
            if header_set.insert(key.clone()) {
                headers.push(key.clone());
            }
        }
        all_flat.push(pairs);
    }

    // Ensure "Name" column (from dict-of-objects) appears first
    if let Some(pos) = headers.iter().position(|h| h == "Name") {
        if pos > 0 {
            let name = headers.remove(pos);
            headers.insert(0, name);
        }
    }

    // Build rows aligned to the unified header list
    let mut rows = Vec::new();
    for flat in &all_flat {
        let map: std::collections::HashMap<&str, &str> =
            flat.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let row: Vec<String> = headers.iter()
            .map(|h| map.get(h.as_str()).unwrap_or(&"").to_string())
            .collect();
        rows.push(row);
    }

    let row_count = rows.len();
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(FileData {
        headers,
        rows,
        row_count,
        file_name,
        file_type: "JSON".to_string(),
    })
}

/// Parse JSONL file (newline-delimited JSON) and return structured data
#[tauri::command]
fn parse_jsonl(file_path: String) -> Result<FileData, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    
    if lines.is_empty() {
        return Err("JSONL file is empty".to_string());
    }
    
    // Parse first line to get headers
    let first_line: serde_json::Value = serde_json::from_str(lines[0])
        .map_err(|e| format!("Failed to parse first line: {}", e))?;
    
    let headers: Vec<String> = if let Some(obj) = first_line.as_object() {
        obj.keys().map(|k| k.to_string()).collect()
    } else {
        return Err("JSONL lines must be objects".to_string());
    };
    
    // Parse all lines
    let mut rows = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let obj: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("Failed to parse line {}: {}", i + 1, e))?;
        
        if let Some(obj_map) = obj.as_object() {
            let row: Vec<String> = headers.iter()
                .map(|h| {
                    obj_map.get(h)
                        .and_then(|v| match v {
                            serde_json::Value::String(s) => Some(s.clone()),
                            serde_json::Value::Number(n) => Some(n.to_string()),
                            serde_json::Value::Bool(b) => Some(b.to_string()),
                            serde_json::Value::Null => Some("".to_string()),
                            _ => Some(v.to_string()),
                        })
                        .unwrap_or_else(|| "".to_string())
                })
                .collect();
            rows.push(row);
        }
    }
    
    let row_count = rows.len();
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    Ok(FileData {
        headers,
        rows,
        row_count,
        file_name,
        file_type: "JSONL".to_string(),
    })
}

/// Export data to CSV format
#[tauri::command]
fn export_csv(file_path: String, headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<String, String> {
    let mut writer = csv::Writer::from_path(&file_path)
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;
    
    writer.write_record(&headers)
        .map_err(|e| format!("Failed to write headers: {}", e))?;
    
    for row in rows {
        writer.write_record(&row)
            .map_err(|e| format!("Failed to write row: {}", e))?;
    }
    
    writer.flush()
        .map_err(|e| format!("Failed to save file: {}", e))?;
    
    Ok(format!("Successfully exported to {}", file_path))
}

/// Export data to JSON format (array of objects)
#[tauri::command]
fn export_json(file_path: String, headers: Vec<String>, rows: Vec<Vec<String>>) -> Result<String, String> {
    let mut json_array = Vec::new();
    
    for row in rows {
        let mut obj = serde_json::Map::new();
        for (i, header) in headers.iter().enumerate() {
            if let Some(value) = row.get(i) {
                obj.insert(header.clone(), serde_json::Value::String(value.clone()));
            }
        }
        json_array.push(serde_json::Value::Object(obj));
    }
    
    let json_string = serde_json::to_string_pretty(&json_array)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    
    fs::write(&file_path, json_string)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    
    Ok(format!("Successfully exported to {}", file_path))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            parse_csv,
            parse_json,
            parse_jsonl,
            export_csv,
            export_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
