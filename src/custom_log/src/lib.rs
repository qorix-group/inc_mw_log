pub use mw_log::Level;
pub use mw_log_subscriber::types::{to_log_value, LogValue};
pub use mw_log_subscriber::MwLogger;
mod macros;

pub fn send_record(level: Level, values: Vec<LogValue>) {
    MwLogger::global().send_record(level as u8, values)
}
