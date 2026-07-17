use std::fs;
use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
pub fn backup_db(app_handle: AppHandle, dest_path: String) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
        
    let db_path = app_dir.join("bloc.db");
    
    fs::copy(&db_path, &dest_path)
        .map_err(|e| format!("Failed to copy database: {}", e))?;
        
    Ok(format!("Database backed up successfully to {}", dest_path))
}

#[tauri::command]
pub fn restore_db(app_handle: AppHandle, source_path: String) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
        
    let db_path = app_dir.join("bloc.db");
    
    fs::copy(&source_path, &db_path)
        .map_err(|e| format!("Failed to restore database: {}", e))?;
        
    Ok("Database restored successfully. Please restart the application.".to_string())
}
