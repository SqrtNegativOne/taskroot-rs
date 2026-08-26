const Database = require('better-sqlite3');
const path = require('path');
const os = require('os');
const dbPath = path.join(os.homedir(), 'AppData', 'Roaming', 'taskroot', 'taskroot.db'); // typical tauri path on windows
try {
    const db = new Database(dbPath, { readonly: true });
    const events = db.prepare('SELECT start_time, end_time, title FROM events LIMIT 20').all();
    console.log(JSON.stringify(events, null, 2));
} catch (e) {
    console.log('Error opening db:', e.message);
}
