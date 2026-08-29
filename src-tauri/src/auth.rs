use crate::db::DbState;
use bcrypt::{hash, verify, DEFAULT_COST};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub role: String, // 'admin' or 'user'
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResponse {
    pub success: bool,
    pub user: Option<User>,
    pub message: String,
}

#[tauri::command]
pub fn check_has_users(state: State<'_, DbState>) -> Result<bool, String> {
    let conn = state.0.lock().unwrap();
    let count: i64 = conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)).unwrap_or(0);
    Ok(count > 0)
}

#[tauri::command]
pub fn setup_admin(state: State<'_, DbState>, username: String, password_raw: String) -> Result<String, String> {
    let conn = state.0.lock().unwrap();
    
    // Safety check: Ensure no users exist before creating the first admin
    let count: i64 = conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0)).unwrap_or(0);
    if count > 0 {
        return Err("Application is already initialized with an Admin account.".to_string());
    }

    let hashed_pw = hash(&password_raw, DEFAULT_COST).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, 'admin')",
        params![username, hashed_pw],
    ).map_err(|e| e.to_string())?;

    Ok("Admin account setup successfully.".to_string())
}

#[tauri::command]
pub fn login(state: State<'_, DbState>, username: String, password_raw: String) -> Result<LoginResponse, String> {
    let conn = state.0.lock().unwrap();
    
    let mut stmt = conn.prepare("SELECT id, username, password_hash, role FROM users WHERE username = ?1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![username]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = row.get(0).map_err(|e| e.to_string())?;
        let db_username: String = row.get(1).map_err(|e| e.to_string())?;
        let hash: String = row.get(2).map_err(|e| e.to_string())?;
        let role: String = row.get(3).map_err(|e| e.to_string())?;

        let valid = verify(&password_raw, &hash).unwrap_or(false);
        if valid {
            Ok(LoginResponse {
                success: true,
                user: Some(User { id, username: db_username, role }),
                message: "Login successful".to_string(),
            })
        } else {
            Ok(LoginResponse {
                success: false,
                user: None,
                message: "Invalid credentials".to_string(),
            })
        }
    } else {
        Ok(LoginResponse {
            success: false,
            user: None,
            message: "Invalid credentials".to_string(),
        })
    }
}

#[tauri::command]
pub fn create_user(state: State<'_, DbState>, current_role: String, username: String, password_raw: String, new_role: String) -> Result<String, String> {
    if current_role != "admin" {
        return Err("Unauthorized: Only Admins can create new users.".to_string());
    }

    let conn = state.0.lock().unwrap();
    
    // Check if user exists
    let exists: bool = conn.query_row("SELECT count(*) FROM users WHERE username = ?1", params![username], |row| {
        let c: i64 = row.get(0)?;
        Ok(c > 0)
    }).unwrap_or(false);

    if exists {
        return Err("User already exists.".to_string());
    }

    let hashed_pw = hash(&password_raw, DEFAULT_COST).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
        params![username, hashed_pw, new_role],
    ).map_err(|e| e.to_string())?;

    Ok("User created successfully.".to_string())
}
