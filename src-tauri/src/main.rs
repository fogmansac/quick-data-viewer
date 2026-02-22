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

/// Parse JSON file (array of objects) and return structured data
#[tauri::command]
fn parse_json(file_path: String) -> Result<FileData, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let data: Vec<serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    if data.is_empty() {
        return Err("JSON file is empty".to_string());
    }
    
    // Extract headers from first object's keys
    let headers: Vec<String> = if let Some(first_obj) = data.first() {
        if let Some(obj) = first_obj.as_object() {
            obj.keys().map(|k| k.to_string()).collect()
        } else {
            return Err("JSON must be an array of objects".to_string());
        }
    } else {
        return Err("JSON file is empty".to_string());
    };
    
    // Convert objects to rows
    let mut rows = Vec::new();
    for item in &data {
        if let Some(obj) = item.as_object() {
            let row: Vec<String> = headers.iter()
                .map(|h| {
                    obj.get(h)
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
