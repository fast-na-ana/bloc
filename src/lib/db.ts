import { invoke } from "@tauri-apps/api/core";

export const dbClient = {
  /**
   * Triggers a manual database backup to the specified destination path.
   * @param destPath The absolute path where the backup should be saved.
   */
  async backupDatabase(destPath: string): Promise<string> {
    try {
      const result = await invoke<string>("backup_db", { destPath });
      return result;
    } catch (error) {
      console.error("Backup failed:", error);
      throw error;
    }
  },

  /**
   * Restores the database from the specified source path.
   * @param sourcePath The absolute path of the backup file to restore.
   */
  async restoreDatabase(sourcePath: string): Promise<string> {
    try {
      const result = await invoke<string>("restore_db", { sourcePath });
      return result;
    } catch (error) {
      console.error("Restore failed:", error);
      throw error;
    }
  },
};
