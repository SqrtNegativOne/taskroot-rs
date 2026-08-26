const Database = require('better-sqlite3');
const path = require('path');
const os = require('os');
const dbPath = path.join(os.homedir(), 'AppData', 'Roaming', 'com.arkma.tauri-app', 'taskroot.db');
try {
    const db = new Database(dbPath, { readonly: true });
    const count = db.prepare('SELECT count(*) as c FROM events').get();
    console.log('Total events in DB:', count.c);
    const timed = db.prepare('SELECT start_time, end_time, title, is_all_day FROM events WHERE is_all_day = 0 OR is_all_day IS NULL LIMIT 5').all();
    console.log('Sample timed events:', JSON.stringify(timed, null, 2));
    const all = db.prepare('SELECT start_time, end_time, title, is_all_day FROM events LIMIT 5').all();
    console.log('Sample all events:', JSON.stringify(all, null, 2));
} catch (e) {
    console.log('Error opening Roaming db:', e.message);
    const dbPath2 = path.join(os.homedir(), 'AppData', 'Local', 'com.arkma.tauri-app', 'taskroot.db');
    try {
        const db2 = new Database(dbPath2, { readonly: true });
        const count = db2.prepare('SELECT count(*) as c FROM events').get();
        console.log('Total events in Local DB:', count.c);
        const timed = db2.prepare('SELECT start_time, end_time, title, is_all_day FROM events WHERE is_all_day = 0 OR is_all_day IS NULL LIMIT 5').all();
        console.log('Sample timed events:', JSON.stringify(timed, null, 2));
    } catch(e2) {
        console.log('Error opening Local db:', e2.message);
    }
}
