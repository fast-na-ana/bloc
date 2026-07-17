use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct FixedAsset {
    pub id: Option<i64>,
    pub name: String,
    pub value: f64,
    pub depreciation_rate: f64, // percentage
}

#[tauri::command]
pub fn create_asset(state: State<'_, DbState>, payload: FixedAsset) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO assets (name, value, depreciation_rate) VALUES (?1, ?2, ?3)",
        params![payload.name, payload.value, payload.depreciation_rate],
    ).map_err(|e| e.to_string())?;

    Ok("Asset registered successfully".to_string())
}

#[tauri::command]
pub fn get_assets(state: State<'_, DbState>) -> Result<Vec<FixedAsset>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, value, depreciation_rate FROM assets")
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            Ok(FixedAsset {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                value: row.get(2)?,
                depreciation_rate: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for item in iter {
        if let Ok(val) = item {
            list.push(val);
        }
    }
    Ok(list)
}
