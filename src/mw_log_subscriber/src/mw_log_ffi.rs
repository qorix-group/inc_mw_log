//
// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0
//

use core::ffi::{c_char, c_uchar, c_uint};
use mw_log::{Level, LevelFilter};
use std::ffi::CString;
use std::fmt;

// Opaque type representing the C++ logger ptr
#[repr(C)]
pub(crate) struct Logger {
    _private: [u8; 0], // Opaque
}

pub(crate) fn mw_log_logger_level(logger: *const Logger) -> LevelFilter {
    let level = unsafe { mw_log_logger_level_internal(logger) };
    log_level_from_ffi(level)
}

pub(crate) fn mw_log_is_log_level_enabled(logger: *const Logger, level: Level) -> bool {
    let level_byte = match level {
        Level::Error => 0x02,
        Level::Warn => 0x03,
        Level::Info => 0x04,
        Level::Debug => 0x05,
        Level::Trace => 0x06,
    };
    unsafe { mw_log_is_log_level_enabled_internal(logger, level_byte) }
}

/// Get the max log level from C++ as a LevelFilter directly
fn log_level_from_ffi(level: u8) -> LevelFilter {
    match level {
        0x00 => LevelFilter::Off,
        0x01 => LevelFilter::Error, // Currently Fatal treated as Error
        0x02 => LevelFilter::Error,
        0x03 => LevelFilter::Warn,
        0x04 => LevelFilter::Info,
        0x05 => LevelFilter::Debug,
        0x06 => LevelFilter::Trace, // Verbose is Trace
        _ => LevelFilter::Info,     // fallback
    }
}

#[repr(C)]
pub struct FfiValue {
    pub tag: u8, // discriminant: 0=i32, 1=u32, 2=i64, 3=u64, 4=f64, 5=bool, 6=str
    pub data: FfiValueData,
}

#[repr(C)]
pub union FfiValueData {
    pub i32_val: i32,
    pub u32_val: u32,
    pub i64_val: i64,
    pub u64_val: u64,
    pub f64_val: f64,
    pub bool_val: bool,
    pub str_ptr: *const c_char,
}

impl fmt::Debug for FfiValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            match self.tag {
                0 => write!(
                    f,
                    "FfiValue {{ tag: 0 (i32), data: {} }}",
                    self.data.i32_val
                ),
                1 => write!(
                    f,
                    "FfiValue {{ tag: 1 (u32), data: {} }}",
                    self.data.u32_val
                ),
                2 => write!(
                    f,
                    "FfiValue {{ tag: 2 (i64), data: {} }}",
                    self.data.i64_val
                ),
                3 => write!(
                    f,
                    "FfiValue {{ tag: 3 (u64), data: {} }}",
                    self.data.u64_val
                ),
                4 => write!(
                    f,
                    "FfiValue {{ tag: 4 (f64), data: {} }}",
                    self.data.f64_val
                ),
                5 => write!(
                    f,
                    "FfiValue {{ tag: 5 (bool), data: {} }}",
                    self.data.bool_val
                ),
                6 => write!(
                    f,
                    "FfiValue {{ tag: 6 (str_ptr), data: {:?} }}",
                    self.data.str_ptr
                ),
                _ => write!(f, "FfiValue {{ tag: {}, data: <unknown> }}", self.tag),
            }
        }
    }
}

impl FfiValue {
    pub fn from_i32(v: i32) -> Self {
        Self {
            tag: 0,
            data: FfiValueData { i32_val: v },
        }
    }
    pub fn from_u32(v: u32) -> Self {
        Self {
            tag: 1,
            data: FfiValueData { u32_val: v },
        }
    }
    pub fn from_i64(v: i64) -> Self {
        Self {
            tag: 2,
            data: FfiValueData { i64_val: v },
        }
    }
    pub fn from_u64(v: u64) -> Self {
        Self {
            tag: 3,
            data: FfiValueData { u64_val: v },
        }
    }
    pub fn from_f64(v: f64) -> Self {
        Self {
            tag: 4,
            data: FfiValueData { f64_val: v },
        }
    }
    pub fn from_bool(v: bool) -> Self {
        Self {
            tag: 5,
            data: FfiValueData { bool_val: v },
        }
    }
    pub fn from_str(s: &str) -> Self {
        let cstr = CString::new(s).unwrap();
        let ptr = cstr.into_raw(); // hand ownership to C++
        Self {
            tag: 6,
            data: FfiValueData { str_ptr: ptr },
        }
    }
}

extern "C" {

    pub(crate) fn mw_log_create_logger(context: *const c_char) -> *const Logger;
    fn mw_log_is_log_level_enabled_internal(logger: *const Logger, level: u8) -> bool;
    fn mw_log_logger_level_internal(logger: *const Logger) -> u8;

    pub(crate) fn mw_log_send_record(
        logger: *const Logger,
        level: u8,
        values: *const FfiValue,
        len: u32,
    );
}
