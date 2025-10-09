// rust_custom_formatter_example.rs
// Demonstrates hiding types when passing them to a backend writer/FFI
// - ExampleDebug trait: custom types implement fmt(&mut dyn CustomWriter)
// - Arg: type-erased pointer + function pointer that knows how to format
// - CustomWriter: backend-facing trait (would call FFI in real backend)
// - Logger::log: accepts a lifetime-bound slice of Arg and a formatter

use std::marker::PhantomData;
use std::ptr::NonNull;

// --- Backend-facing writer trait -------------------------------------------------
// In a real system this trait would be implemented by a type that holds an opaque
// FFI-backed log stream. Each method would call into C++/C FFI overloads.
pub trait CustomWriter {
    fn write_str(&mut self, s: &str);
    fn write_u32(&mut self, v: u32);
    fn write_u64(&mut self, v: u64);
    // add more primitive handlers as needed (i8, i16, f32, etc.)
}

// Simple demo implementation of CustomWriter that prints to stdout.
// Replace these bodies with extern "C" FFI calls in your backend.
pub struct BackendWriter;
impl CustomWriter for BackendWriter {
    fn write_str(&mut self, s: &str) {
        // In real backend: call into C++ like `LogInfo() << s` via FFI
        print!("{}", s);
    }

    fn write_u32(&mut self, v: u32) {
        // In real backend: call FFI overload that takes unsigned int
        print!("{}", v);
    }

    fn write_u64(&mut self, v: u64) {
        print!("{}", v);
    }
}

// --- ExampleDebug trait for user-defined types -------------------------------
pub trait ExampleDebug: Sized {
    // Format the custom type using the backend writer
    fn fmt(&self, w: &mut dyn CustomWriter);

    // Generic trampoline used when we have an erased pointer + function pointer
    fn transpassi_into_fmt(data: NonNull<()>, w: &mut dyn CustomWriter) {
        let data = data.as_ptr() as *const Self;
        // SAFETY: caller must guarantee `data` points to a valid `Self` for lifetime
        unsafe { (&*data).fmt(w) }
    }
}

// --- Arg: type-erased argument that knows how to format itself ---------------
pub struct Arg<'a> {
    value: NonNull<()>,
    formatting: unsafe fn(NonNull<()>, &mut dyn CustomWriter),
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> Arg<'a> {
    pub fn new<T>(data: &'a T, formatting: unsafe fn(NonNull<()>, &mut dyn CustomWriter)) -> Self {
        Self {
            value: NonNull::new(data as *const T as *mut ()).unwrap(),
            formatting,
            _lifetime: PhantomData,
        }
    }

    pub fn new_debug_type<T: ExampleDebug>(item: &'a T) -> Self {
        // Use the generic trampoline as formatting function
        Self::new(item, ExampleDebug::transpassi_into_fmt::<T>)
    }

    // Convenience constructors for primitive types that call the appropriate
    // formatting function (they forward to the same unsafe function pointer type).
    pub fn from_u32(value: &'a u32) -> Self {
        // Build a formatting function that will cast back to u32 and call writer
        unsafe fn fmt_u32(data: NonNull<()>, w: &mut dyn CustomWriter) {
            let p = data.as_ptr() as *const u32;
            let v = unsafe { *p };
            w.write_u32(v);
        }
        Self::new(value, fmt_u32)
    }

    pub fn from_u64(value: &'a u64) -> Self {
        unsafe fn fmt_u64(data: NonNull<()>, w: &mut dyn CustomWriter) {
            let p = data.as_ptr() as *const u64;
            let v = unsafe { *p };
            w.write_u64(v);
        }
        Self::new(value, fmt_u64)
    }

    pub fn from_str(value: &'a &str) -> Self {
        unsafe fn fmt_str(data: NonNull<()>, w: &mut dyn CustomWriter) {
            let p = data.as_ptr() as *const &str;
            let v = unsafe { &*p };
            w.write_str(v);
        }
        Self::new(value, fmt_str)
    }

    // Call the stored formatting function with a backend writer
    pub fn fmt(&self, w: &mut dyn CustomWriter) {
        unsafe { (self.formatting)(self.value, w) }
    }
}

// --- Example user type ------------------------------------------------------
pub struct MycustomType {
    a: u32,
    b: u64,
    c: &'static str,
}

impl ExampleDebug for MycustomType {
    fn fmt(&self, w: &mut dyn CustomWriter) {
        // The implementation uses the writer to output pieces. In a real
        // backend these could map to C++ overloads.
        w.write_str("MycustomType { a: ");
        w.write_u32(self.a);
        w.write_str(", b: ");
        w.write_u64(self.b);
        w.write_str(", c: ");
        w.write_str(self.c);
        w.write_str(" }");
    }
}

// --- Logger that takes a slice of Arg and a format string (very small demo) ---
pub struct Logger;
impl Logger {
    pub fn log<'a>(&self, fmt_str: &str, args: &[Arg<'a>]) {
        // In a real implementation 'fmt_str' would be parsed and tokens
        // matched with `args`. Here we'll do a tiny toy parser that treats
        // '{}' tokens as argument slots in order.

        let mut writer = BackendWriter;
        let mut parts = fmt_str.split("{}");
        if let Some(first) = parts.next() {
            writer.write_str(first);
        }
        for (i, part) in parts.enumerate() {
            if i < args.len() {
                args[i].fmt(&mut writer);
            } else {
                writer.write_str("<missing>");
            }
            writer.write_str(part);
        }
        // newline for demo
        writer.write_str("\n");
    }
}

// --- Small macro that creates local vars and calls `Logger::log` -------------
// This macro is intentionally minimal to demonstrate the flow. A production
// macro would need to validate argument counts, support named args, format
// specifiers like `{:?}`, widths, etc.
#[macro_export]
macro_rules! my_log {
    ($logger:expr, $fmt:expr $(, $arg:expr)*) => {
        {
            // Create local variables to ensure they live long enough for Arg
            $( let __arg_val = $arg; )*
            // Build Arg slice
            let mut __args_vec = Vec::new();
            $(
                // Infer primitive vs debug by simple trait bound attempt is hard in macro,
                // so the caller should use explicit constructors when needed.
                __args_vec.push(Arg::from_str(&__arg_val));
            )*
            // Convert Vec<Arg> -> Vec<Arg> (lifetime ties to current scope) and call
            $logger.log($fmt, &__args_vec);
        }
    };
}

// --- Demo ---------------------------------------------------------------
fn main() {
    let custom = MycustomType { a: 42, b: 100, c: "Hello, world!" };
    let x: u32 = 7;
    let s: &str = "a string";

    // Manually build Arg slice and call Logger
    let arg_custom = Arg::new_debug_type(&custom);
    let arg_x = Arg::from_u32(&x);
    let arg_s = Arg::from_str(&s);

    let logger = Logger;
    logger.log("custom: {} | x: {} | s: {}", &[arg_custom, arg_x, arg_s]);

    // Using the tiny macro (note: macro currently treats args as &str)
    // my_log!(&logger, "macro: {} {}", s, s);
}

/*
Notes & next steps:
- The macro here is minimal; a full macro should analyze format string at compile
  time and emit Arg constructors with the correct types (u32 vs debug types).
- For FFI: implement a struct like `CxxBackendWriter` that holds an opaque
  pointer to your C++ log stream and implement `CustomWriter` by calling
  extern "C" functions like `log_stream_write_u32(ptr, v)`.
- If you want to support lifetime-free static messages (no references), you can
  allocate arguments into a small slab/arena and store pointers with 'static.
- To support `{:?}`, `{:x}`, widths, etc. you can pass a small descriptor along
  with each Arg (enum or bitflags) describing how it should be printed.
*/
