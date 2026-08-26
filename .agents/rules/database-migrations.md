---
name: database-migrations
description: Use this rule when you need to modify the SQLite database schema, add new columns, or migrate data.
trigger: model_decision
---

# Database Schema & Migrations

Because the schema is initialized using `CREATE TABLE IF NOT EXISTS` guards in `src-tauri/src/db/mod.rs`, adding new columns to existing tables won't automatically update older databases.

For local development (since the app is single-user), you should apply manual schema migrations directly to the SQLite file using `bun:sqlite` instead of setting up a complex migration framework. 

The database is located in the OS app data directory (e.g., `%APPDATA%\com.arkma.tauri-app\taskroot.db` on Windows).

## Example Bun Migration Script

You can create a temporary file like `migrate.js` in the project root to perform migrations:

```javascript
import { Database } from "bun:sqlite";

// Update this path to match your OS app data location
const dbPath = process.env.APPDATA + "/com.arkma.tauri-app/taskroot.db";
const db = new Database(dbPath);

try {
    db.run("ALTER TABLE events ADD COLUMN color TEXT;");
    console.log("Migration successful");
} catch(e) {
    console.error("Migration failed:", e.message);
} finally {
    db.close();
}
```

Run the script with:
```bash
bun run migrate.js
```
*Note: Always remember to update the Rust structs and inline `CREATE TABLE` queries in `src-tauri/src/db/mod.rs` after running a local migration, so new users get the correct schema out-of-the-box.*
