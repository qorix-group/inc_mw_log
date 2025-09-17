// use std::ffi::CString;

// /// Typed log values to avoid string-only logging
#[repr(C)]
#[derive(Debug)]
pub enum LogValue {
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
    Str(String), // CString pointer
}

// /// A structured log record passed to C++
#[repr(C)]
#[derive(Debug)]
pub struct LogRecord {
    pub level: u8, // maps to log::Level
    pub target: *const i8,
    pub args: *const LogValue,
    pub len: usize,
}

pub fn to_log_value<T: IntoLogValue>(val: T) -> LogValue {
    val.into_log_value()
}

pub trait IntoLogValue {
    fn into_log_value(self) -> LogValue;
}

impl IntoLogValue for i32 {
    fn into_log_value(self) -> LogValue {
        LogValue::I32(self)
    }
}
impl IntoLogValue for i64 {
    fn into_log_value(self) -> LogValue {
        LogValue::I64(self)
    }
}
impl IntoLogValue for u32 {
    fn into_log_value(self) -> LogValue {
        LogValue::U32(self)
    }
}
impl IntoLogValue for f64 {
    fn into_log_value(self) -> LogValue {
        LogValue::F64(self)
    }
}
impl IntoLogValue for bool {
    fn into_log_value(self) -> LogValue {
        LogValue::Bool(self)
    }
}
impl IntoLogValue for &str {
    fn into_log_value(self) -> LogValue {
        LogValue::Str(self.to_string())
    }
}
impl IntoLogValue for String {
    fn into_log_value(self) -> LogValue {
        LogValue::Str(self)
    }
}
