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

pub mod mw_log_ffi;
pub mod types;

use crate::mw_log_ffi::*;
use crate::types::{LogRecord, LogValue};

use core::ffi::c_char;
use core::fmt::{self, Write};
use mw_log::{Level, Log, Metadata, Record};
use std::ffi::CString;
use std::mem::MaybeUninit;

const MSG_SIZE: usize = 512;

/// Builder for the MwLogger
pub struct MwLoggerBuilder {
    context: Option<CString>,
}

impl MwLoggerBuilder {
    pub fn new() -> Self {
        Self { context: None }
    }

    /// Builds the MwLogger with the specified context and configuration and returns it.
    pub fn build<const SHOW_MODULE: bool, const SHOW_FILE: bool, const SHOW_LINE: bool>(
        self,
    ) -> MwLogger {
        let context_cstr = self.context.unwrap_or(CString::new("DFLT").unwrap());
        let c_logger_ptr = unsafe { mw_log_create_logger(context_cstr.as_ptr() as *const _) };
        MwLogger {
            ptr: c_logger_ptr,
            log_fn: log::<SHOW_MODULE, SHOW_FILE, SHOW_LINE>,
        }
    }

    /// Builds and sets the MwLogger as the default logger with the specified configuration.
    pub fn set_as_default_logger<
        const SHOW_MODULE: bool,
        const SHOW_FILE: bool,
        const SHOW_LINE: bool,
    >(
        self,
    ) {
        let logger = self.build::<SHOW_MODULE, SHOW_FILE, SHOW_LINE>();
        mw_log::set_max_level(mw_log_logger_level(logger.ptr));
        mw_log::set_boxed_logger(Box::new(logger))
            .expect("Failed to initialize MwLogger as default logger - logger may already be set");
    }

    /// Sets the context for currently build logger.
    pub fn context(mut self, context: &str) -> Self {
        self.context = Some(CString::new(context).expect(
            "Failed to create CString:
             input contains null bytes",
        ));
        self
    }
}

struct BufWriter<const BUF_SIZE: usize> {
    buf: [MaybeUninit<u8>; BUF_SIZE],
    pos: usize,
}

impl<const BUF_SIZE: usize> BufWriter<BUF_SIZE> {
    fn new() -> Self {
        Self {
            buf: [MaybeUninit::uninit(); BUF_SIZE],
            pos: 0,
        }
    }

    /// Returns a slice to filled part of buffer. This is not null-terminated.
    fn as_c_str(&self) -> &[c_char] {
        unsafe { core::slice::from_raw_parts(self.buf.as_ptr().cast::<c_char>(), self.len()) }
    }

    fn len(&self) -> usize {
        self.pos
    }

    fn revert_pos(&mut self, cnt: usize) {
        self.pos = self.pos.saturating_sub(cnt);
    }

    // Finds the right index (around requested `index`) to trim utf8 string to a valid char boundary
    fn floor_char_boundary(view: &str, index: usize) -> usize {
        if index >= view.len() {
            view.len()
        } else {
            let lower_bound = index.saturating_sub(3);
            let new_index = view.as_bytes()[lower_bound..=index]
                .iter()
                .rposition(|b| Self::is_utf8_char_boundary(*b));

            // SAFETY: we know that the character boundary will be within four bytes
            unsafe { lower_bound + new_index.unwrap_unchecked() }
        }
    }

    const fn is_utf8_char_boundary(i: u8) -> bool {
        // This is bit magic equivalent to: b < 128 || b >= 192
        (i as i8) >= -0x40
    }
}

impl<const BUF_SIZE: usize> Write for BufWriter<BUF_SIZE> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let to_write = bytes.len().min(self.buf.len() - self.pos - 1); // Cutting write, instead error when we don't fit
        let bounded_to_write = Self::floor_char_boundary(s, to_write);

        if bounded_to_write == 0 {
            return Err(fmt::Error);
        }

        let dest = self.buf[self.pos..self.pos + bounded_to_write]
            .as_mut_ptr()
            .cast::<u8>();

        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bounded_to_write) };

        self.pos += bounded_to_write;
        Ok(())
    }
}

use crate::mw_log_ffi::{mw_log_send_record, FfiValue};
use std::sync::Mutex;

#[derive(Debug)]
pub struct MwLogger {
    ptr: *const Logger,
    log_fn: fn(&mut BufWriter<MSG_SIZE>, &Record),
}

// SAFETY: The underlying C++ logger is assumed to be safe to change thread
unsafe impl Send for MwLogger {}

// SAFETY: The underlying C++ logger is assumed to be thread-safe.
unsafe impl Sync for MwLogger {}

use std::sync::OnceLock;

static GLOBAL_LOGGER: OnceLock<Mutex<MwLogger>> = OnceLock::new();

impl MwLogger {
    pub fn set_global(self) {
        GLOBAL_LOGGER
            .set(Mutex::new(self))
            .expect("Global logger already initialized");
    }

    pub fn global() -> std::sync::MutexGuard<'static, MwLogger> {
        GLOBAL_LOGGER
            .get()
            .expect("Global logger not initialized")
            .lock()
            .expect("Poisoned global logger mutex")
    }

    pub fn send_record(&self, level: u8, values: Vec<LogValue>) {
        // Prepare ffi_values and a list of allocated CString pointers for cleanup
        let mut ffi_values: Vec<FfiValue> = Vec::with_capacity(values.len());
        let mut owned_cstrings: Vec<*mut c_char> = Vec::new();

        for v in values {
            match v {
                LogValue::I32(x) => {
                    ffi_values.push(FfiValue {
                        tag: 0,
                        data: FfiValueData { i32_val: x },
                    });
                }
                LogValue::U32(x) => {
                    ffi_values.push(FfiValue {
                        tag: 1,
                        data: FfiValueData { u32_val: x },
                    });
                }
                LogValue::I64(x) => {
                    ffi_values.push(FfiValue {
                        tag: 2,
                        data: FfiValueData { i64_val: x },
                    });
                }
                LogValue::U64(x) => {
                    ffi_values.push(FfiValue {
                        tag: 3,
                        data: FfiValueData { u64_val: x },
                    });
                }
                LogValue::F64(x) => {
                    ffi_values.push(FfiValue {
                        tag: 4,
                        data: FfiValueData { f64_val: x },
                    });
                }
                LogValue::Bool(b) => {
                    let val: u32 = if b { 1 } else { 0 };
                    ffi_values.push(FfiValue {
                        tag: 5,
                        data: FfiValueData { u32_val: val },
                    });
                }
                LogValue::Str(s) => {
                    let c = CString::new(s).expect("CString::new failed");
                    let ptr = c.into_raw(); // leak to FFI temporarily
                    owned_cstrings.push(ptr);

                    ffi_values.push(FfiValue {
                        tag: 6,
                        data: FfiValueData { str_ptr: ptr },
                    });
                }
            }
        }
        println!(
            "Logging via FFI with level: {}, values: {} content {:?}",
            level,
            ffi_values.len(),
            ffi_values
        );
        // Call into C++
        unsafe {
            mw_log_send_record(
                self.ptr,
                level,
                ffi_values.as_ptr(),
                ffi_values.len() as u32,
            );
        }

        // Reclaim CString memory after C++ call
        for raw in owned_cstrings {
            unsafe {
                let _ = CString::from_raw(raw);
            }
        }
    }
}

impl Log for MwLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        mw_log_is_log_level_enabled(self.ptr, metadata.level())
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
    }

    fn flush(&self) {
        // No-op
    }
}

fn log<const SHOW_MODULE: bool, const SHOW_FILE: bool, const SHOW_LINE: bool>(
    msg_writer: &mut BufWriter<MSG_SIZE>,
    record: &Record,
) {
    if SHOW_FILE || SHOW_LINE || SHOW_MODULE {
        let _ = write!(msg_writer, "[");
        if SHOW_MODULE {
            if let Some(module) = record.module_path() {
                let _ = write!(msg_writer, "{}:", module);
            }
        }
        if SHOW_FILE {
            if let Some(file) = record.file() {
                let _ = write!(msg_writer, "{}:", file);
            }
        }
        if SHOW_LINE {
            if let Some(line) = record.line() {
                let _ = write!(msg_writer, "{}:", line);
            }
        }

        msg_writer.revert_pos(1);
        let _ = write!(msg_writer, "] ");
    }

    let _ = msg_writer.write_fmt(*record.args());
}
