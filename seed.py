import sqlite3
import os

path = r'C:\Users\Abd\AppData\Roaming\com.abd.tauri-app\bloc.db'
os.makedirs(os.path.dirname(path), exist_ok=True)
conn = sqlite3.connect(path)
conn.execute("CREATE TABLE IF NOT EXISTS accounts (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, type TEXT NOT NULL, credit_limit REAL DEFAULT 0, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)")
conn.execute("INSERT INTO accounts (name, type) VALUES ('Test Account', 'customer')")
conn.commit()
conn.close()
print("Success")
