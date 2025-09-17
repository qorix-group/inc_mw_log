pub use mw_log::Level;
pub use mw_log_subscriber::types::{to_log_value, LogValue};
pub use mw_log_subscriber::MwLogger;
mod macros;
// pub fn send_record<L: Into<mw_log::Level>>(level: L, values: impl Into<Vec<LogValue>>) {
//     let level = level.into();
//     let values: Vec<LogValue> = values.into();

//     // delegate to subscriber (FFI layer)
//     mw_log_subscriber::ffi_send(level, values);
// }
pub fn send_record(level: Level, values: Vec<LogValue>) {
    MwLogger::global().send_record(level as u8, values)
}
