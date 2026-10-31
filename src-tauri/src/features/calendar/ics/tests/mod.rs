//! I1–I3 round-trip tests over real Radicale resources.
//!
//! The fixtures are byte-for-byte copies of files under `C:\radicale`; the
//! exception tests build a multi-component resource in-memory instead (plan §13).

mod alarms;
mod fixtures;
mod lines;
mod read;
mod write;
