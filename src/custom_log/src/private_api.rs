// pub fn set_global_logger(ptr: *const ffi::Logger) { /* store in AtomicPtr */ }
// pub fn get_global_logger() -> *const ffi::Logger { /* load AtomicPtr */ }

// pub trait LogValue {
//     fn push(self, stream: *mut ffi::LogStreamHandle);
// }

// impl LogValue for &str { fn push(self, s) { unsafe { ffi::mw_log_push_str(s, self.as_ptr() as *const i8, self.len() as u32) } } }
// impl LogValue for i32 { fn push(self, s) { unsafe { ffi::mw_log_push_i32(s, self) } } }
// impl LogValue for i64 { fn push(self, s) { unsafe { ffi::mw_log_push_i64(s, self) } } }
// impl LogValue for u32 { fn push(self, s) { unsafe { ffi::mw_log_push_u32(s, self) } } }
// impl LogValue for f32 { fn push(self, s) { unsafe { ffi::mw_log_push_f32(s, self) } } }
// impl LogValue for f64 { fn push(self, s) { unsafe { ffi::mw_log_push_f64(s, self) } } }
// // ... other primitive impls

// pub fn begin_and_push<F>(logger: *const ffi::Logger, level: u8, fmt: &str, push_args: F)
// where F: FnOnce(*mut ffi::LogStreamHandle)
// {
//     unsafe {
//         let s = ffi::mw_log_begin(logger, level);
//         if s.is_null() { return; }
//         // push the format string first (optional)
//         ffi::mw_log_push_str(s, fmt.as_ptr() as *const i8, fmt.len() as u32);
//         push_args(s);
//         ffi::mw_log_finish(s);
//     }
// }
