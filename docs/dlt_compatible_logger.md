# Rust ↔ C++ Typed Logging for DLT

> Research & proof-of-concept: how typed log payloads flow from a C++ `LogError() << "Test" << 32;` style API into the DLT daemon and viewer, why type information matters, how Rust `log`/`tracing`/`defmt` behave, and two interoperable approaches for a Rust wrapper that preserves type info.

---

## Executive summary

* **DLT protocol** carries typed arguments: each argument encoded with a *type tag* (e.g. `STRG`, `SINT`, `U64`, `FLOAT`) and binary representation. This lets the daemon & viewer decode and present values without guessing.
* The Rust `log` crate **stringifies** (via `fmt::Arguments`) and therefore loses type information; `tracing` preserves typed fields through `Visit`/`Record` APIs. `defmt` is an embedded-focused system that logs compact, typed frames by encoding format+arguments and decoding them later with the symbol table — it *also* preserves type semantics but uses a different transport/codec.
* Our **C++ logger** (via `Recorder::Log(slot, T)`) preserves type info by having typed overloads that write a type tag + bytes into the DLT message buffer.
* Two practical ways to let Rust code log typed fields into your C++ recorder without stringifying:

  1. **C ABI shim + Rust FFI wrapper** — a thin C API around C++ typed functions (`logger_push_i32`, `logger_push_str`, etc.), and a Rust builder/macro that calls those functions in order. This preserves type tags because the C++ side performs encoding.
  2. **`tracing` Subscriber → FFI** — implement a `tracing::Subscriber` (or `Layer`) that receives typed fields from `tracing` and forwards them via FFI to the C++ recorder; integrates naturally with Rust structured logging.

This document contains detailed theory, minimal code snippets (C++ and Rust).

---

## Part A — DLT protocol & role of type information

### What type information means in DLT

* Each argument in a DLT message is encoded as: **\[type tag]\[length (if needed)]\[raw bytes]**.
* Type tags indicate how to interpret bytes: signed/unsigned integer of specific width, float, string, raw blob, ASCII/UTF-8, boolean, etc.
* The **standard header** (protocol header) includes message type, ECU ID, timestamp, length; the **extended header** includes application/context IDs and log level.

### Why it's needed

* Allows viewer to display typed values correctly (numeric vs string vs raw hex).
* Enables rich filtering in viewer (e.g., `value > 100`) and accurate binary transfers (no locale/formatting ambiguity).
* Avoids lossiness of formatting-based logging (no parsing back from strings).

### Where type info originates

* On the sender side, e.g. your C++ `Recorder::Log(const SlotHandle&, T)`, each overload *prepends a DLT type tag* and appends binary representation to the payload buffer before the message is sent to daemon.


### How Type Information Is Captured by the C++ DLT User Library

##### Step 1.1 – Logging Macro
- When you call something like:

```DLT_LOG(ctx, DLT_LOG_ERROR, DLT_STRING("Test"), DLT_INT(32));```


- the internal flow goes like this:

-- Start the log message:

```dlt_user_log_write_start(&ctx, &log_data, loglevel);```


 Allocates internal buffer, inserts headers, etc.

-- Write a string argument:

```dlt_user_log_write_string(&log_data, "Test");```


Internally, this writes:

The type tag DLT_TYPE_INFO_STRG

Null-terminated ASCII content

Maybe the length and encoding flags


-- Write an integer argument:

```dlt_user_log_write_int(&log_data, 32);```


This dispatches to the right integer size, e.g., dlt_user_log_write_int32(), which writes:

A tag such as DLT_TYPE_INFO_SINT combined with size bits (e.g., 32-bit)

The binary representation of the integer

-- Finish & send:

```dlt_user_log_write_finish(&log_data);```


This finalizes headers and sends the complete buffer to dlt-daemon over Unix or TCP socket.
File : ```src/lib/dlt_user.c```

The payload now contains type-tagged arguments in binary form—ready for parsing by the daemon and viewer.

##### Step 1.2 – Message Construction

Each log message consists of:

###### Standard Header (dlt_protocol.h)
Contains: message length, MSIN (message type info), ECU ID, session ID, timestamp, etc.

- Built in ```dlt_user_log_send()``` (→ ```dlt_user.c```).

###### Extended Header
Contains: message type (log/trace), log level, application ID, context ID.

- Added in ```dlt_user_log_send()```.

###### Payload

- Encoded arguments, e.g.:

    - DLT_STRING("Test") → writes type info (DLT_TYPE_INFO_STRG) + string length + bytes.

    - DLT_INT(32) → writes type info (DLT_TYPE_INFO_SINT) + integer bytes.

- Functions: ```dlt_user_log_write_string()```, ```dlt_user_log_write_int()```, etc.

- These call lower-level helpers like ```dlt_user_log_write_raw_formatted()```.

Thus the verbose standard + extended header + typed payload is prepared in a buffer.

##### Step 1.3 – Transmission to Daemon

- Once built, the message is sent via:

    - Unix socket (AF_UNIX) to /tmp/dlt (local IPC) OR

    - TCP/IP (Ethernet) if configured.

- Function: dlt_user_log_send() → calls dlt_user_log_send_to_daemon().
- Files: ```dlt_user.c, dlt_user_log.c.```

## How the DLT Daemon Forwards Typed Messages

On the daemon side:

##### Step 2.1 – Daemon Receives Data
- It receives raw message buffers (standard headers + extended headers + typed payload).
- The daemon process (dlt-daemon) listens on:

    - Local Unix domain socket (from applications).

    - Optional TCP port (for external clients).

- Functions: ```dlt_daemon_process_user_message()``` → handles incoming user messages.

- File: ```src/daemon/dlt_daemon.c```.

##### Step 2.2 – Parsing Message

- The daemon does not “re-interpret” payload deeply, but it:

- Validates standard header (```dlt_message_header_read()``` in ```dlt_common.c```).

- Extracts ECU ID, app ID, context ID, timestamp.

- Can filter or control based on configuration (```dlt.conf```).

- Extended header and payload are generally passed through intact to clients.

##### Step 2.3 – Forwarding Over Ethernet

- If configured, the daemon forwards the entire DLT message (standard header + extended header + payload) to connected TCP clients (e.g., DLT Viewer).

- Function: ```dlt_daemon_client_send()```

- File: ```src/daemon/dlt_daemon_client.c.```

## DLT Viewer: Decoding Typed Payloads
- DLT Viewer (or ```dlt-receive```) connects to the daemon via TCP.
- Receives full DLT message.
- The DLT Viewer—whether GUI or dlt-receive—parses the incoming messages:
    - It reads the standard header (length, type, ECU ID, timestamp).
    - It reads the extended header (application ID, context ID, log level).
    - Then it reads each argument from the payload:
        - This works because payload arguments carry data type information (DLT_TYPE_INFO_*).
        - Examines the type tag (e.g., DLT_TYPE_INFO_STRG, DLT_TYPE_INFO_SINT)
        - Reads the correct number of bytes
- Interprets and displays the value accordingly (strings, numbers, raw, etc.)
- This is where the type tagging truly pays off—viewer shows typed values, supports filtering/searching on integers, raw bytes, etc.

---

## Part B — How `log`, `tracing`, and `defmt` behave

### `log` crate

* `log::info!()` uses `format_args!()` (i.e. `fmt::Arguments`) producing formatted text when a backend calls `record.args()`. Most backends receive a single formatted `fmt::Arguments`/string; the type information is lost since formatting runs before the backend.
* Therefore `log` cannot preserve typed argument semantics by default.

### `tracing` crate

* `tracing` records **structured key-value fields**; each field is separately visited using the `Visit` trait (`record_i64`, `record_u64`, `record_str`, ...). Backends (subscribers) implement `Visit` to receive typed values.
* This design makes `tracing` a natural fit if you want typed fields forwarded to a DLT-style recorder.

### `defmt` crate

* `defmt` (embedded) compiles a compact tokenized representation of format strings and argument values at compile time and sends small frames to the host; the host tool uses a symbol table to reconstruct messages. `defmt` *does* retain type information in a compact encoded way — but it is a different ecosystem (not DLT).

---
## Part C — C++ logger flow

### Simplified call flow (example)

```
App: LogError() << "Test" << 32;
  operator<< (template) -> Log(value) overload -> LogRecorder(value) -> CallOnRecorder(&Recorder::Log, slot, value)
    -> Recorder::Log(const SlotHandle&, const char*)  // for string
    -> Recorder::Log(const SlotHandle&, int32_t)      // for integer
      -> writes type tag + bytes into user buffer
      -> ```dlt_user_log_send()``` pushes [StdHdr][ExtHdr][Payload...] to socket -> dlt-daemon

```

### Example `Recorder::Log` implementation

```cpp

- calling LogFatal() returns a LogStream tied to severity = FATAL.
- In log_stream.h
    -   LogStreamSupports<T>() is a type trait that restricts which types are valid (string, int, float, etc.).
    -   Log(value) appends the argument to the payload buffer.
- Log(value) overloads
    LogStream& Log(const char* s);
    LogStream& Log(int32_t i);
    LogStream& Log(uint32_t u);
    LogStream& Log(float f);
    - Each overload converts the C++ type into a known format, then calls: ```return LogRecorder(value);```
- LogRecorder(const T value)
    This function bridges to a Recorder object, which is responsible for actually serializing the data into the internal DLT message buffer.
    Inside it calls : ```return CallOnRecorder(&Recorder::Record, value);```
- CallOnRecorder
    Template dispatcher that takes a pointer-to-member function of Recorder and invokes it with arguments.
    template<typename ReturnValue, typename... ArgsOfFunction, typename... ArgsPassed>
    ReturnValue CallOnRecorder(ReturnValue (Recorder::*arbitrary_function)(ArgsOfFunction...) noexcept,
                           ArgsPassed&&... args) noexcept {
    return (recorder_.*arbitrary_function)(std::forward<ArgsPassed>(args)...);
    }
- recorder_ is the actual Recorder instance inside LogStream.
    It guarantees the right overload of Recorder::Record is called.
    For each argument, the FileRecorder::Log() method is called, which uses the LogData() helper.
- LogData() calls DLTFormat::Log(payload, data) for each argument.
    For each argument:
    Type Information is encoded (e.g., string, uint32).
    Data Payload is encoded (e.g., bytes of "Test", bytes of 32).
- LogData(score::mw::log::detail::VerbosePayload& payload,
                                                const Resolution data,
                                                const score::mw::log::detail::IntegerRepresentation repr,
                                                const std::uint32_t type,
                                                TypeLength type_length)
    This function in dltformat pushes the data to store().
- Store(score::mw::log::detail::VerbosePayload& payload, T... data_for_payload)
    This is a variadic template function: it can accept any number and any type of arguments after payload.
    It checks if all the provided data will fit in the payload buffer (WillMessageFit).
    If so, it serializes each argument into the payload using DoFor, which calls ToByteView for each argument and writes the bytes into the buffer.
    Returns Added if successful, NotAdded otherwise.
    The use of template <typename... T> and T... data_for_payload means you can pass any number and any type of arguments.
    The function is designed to handle multiple arguments, serialize them, and store them in the payload buffer.
    The function will serialize each in order each argument passed.

```

---

## Part D — Approach #1: C ABI shim + Rust FFI wrapper

### Design

* Create a small C-compatible API that exposes typed appenders and message begin/finish.
* Shim calls into your C++ `LogStream` / `Recorder` overloads. The shim is `extern "C"` C++ functions.
* Rust binds to C ABI via `extern "C"` and provides a typed, safe builder + macros to preserve type info.

### Benefits

* Minimal change on C++ side; leverages existing typed `Recorder::Log` overloads.
* Rust does not stringify values.
* Deterministic, easy mapping of Rust primitive types to C++ typed functions.

### C++ shim (files)

* `logger_c_api.h` — declarations
* `logger_c_api.cpp` — definitions (calls into `LogStream`/`Logger`)

```cpp
// logger_c_api.h
#ifdef __cplusplus
extern "C" {
#endif

typedef struct LoggerHandle LoggerHandle;
typedef struct LogStreamHandle LogStreamHandle;

LoggerHandle* logger_create(const char* app, const char* ctx);
void logger_destroy(LoggerHandle*);
LogStreamHandle* logger_begin(LoggerHandle*, int level);
void logger_push_str(LogStreamHandle*, const char* ptr, uint32_t len);
void logger_push_i32(LogStreamHandle*, int32_t v);
void logger_push_u32(LogStreamHandle*, uint32_t v);
void logger_push_i64(LogStreamHandle*, int64_t v);
void logger_push_f32(LogStreamHandle*, float v);
void logger_push_f64(LogStreamHandle*, double v);
void logger_push_raw(LogStreamHandle*, const uint8_t* ptr, uint32_t len);
void logger_finish(LogStreamHandle*);

#ifdef __cplusplus
}
#endif
```

```cpp
// logger_c_api.cpp (implementation sketch)
#include "logger_c_api.h"
#include "logging.h" // your LogStream/Logger types

struct LoggerHandle { std::unique_ptr<Logger> logger; };
struct LogStreamHandle { std::unique_ptr<LogStream> stream; };

LoggerHandle* logger_create(const char* app, const char* ctx) {
    auto h = new LoggerHandle();
    h->logger = Logger::Create(app, ctx);
    return h;
}

LogStreamHandle* logger_begin(LoggerHandle* h, int lvl) {
    auto s = new LogStreamHandle();
    s->stream = std::make_unique<LogStream>(h->logger->Begin((LogLevel)lvl));
    return s;
}

void logger_push_str(LogStreamHandle* s, const char* ptr, uint32_t len) { s->stream->Log(ptr, len); }
void logger_push_i32(LogStreamHandle* s, int32_t v) { s->stream->Log(v); }
void logger_finish(LogStreamHandle* s) { if (!s) return; s->stream->Finish(); delete s; }

// logger_destroy/etc
```

### Rust wrapper (high-level)

* `ffi.rs`: raw `extern "C"` bindings
* `logger.rs`: safe `Logger`/`LogBuilder` types that call into FFI
* `macros.rs`: ergonomics (e.g., `log_fatal!(logger, "Test", 32)`)

See earlier message for a full `ffi.rs` and `LogBuilder` example.

### POC instructions (Approach #1)

1. Implement C++ shim and compile as a static/shared lib (`liblogger_c_api.a` / `.so`).
2. Create Rust crate `rust_logger_ffi` linking to that library (set `cargo:rustc-link-lib` in `build.rs` or `cargo:rustc-link-search`).
3. Write simple Rust `main` that uses the `Logger` wrapper and calls typed methods and macros.
4. Run native `dlt-daemon` and `dlt-viewer` (or `dlt-receive`) to verify the messages show typed fields.

**Files to create (POC1)**

```
cpp/
  logger_c_api.h
  logger_c_api.cpp
  CMakeLists.txt
rust/
  Cargo.toml
  src/ffi.rs
  src/lib.rs (Logger wrapper)
  src/main.rs (demo)
```

Build & test steps (quick)

```bash
# Build C++ shim
mkdir build && cd build
cmake .. && make
# produces liblogger_c_api.a

# Build Rust
cd ../rust
cargo build --release
# Run daemon viewer and example
./target/release/rust_demo
```

---

## Part E — Approach #2: `tracing` subscriber + FFI

### Idea

* Implement a `tracing` `Subscriber` (or `Layer`) that receives typed fields via `Visit::record_*` methods and forwards them to C++ via the same C ABI shim or a direct FFI channel.
* This gives idiomatic Rust usage: `tracing::info!(foo = 42, bar = "ok")`, preserving types.

### Subscriber skeleton (Rust)

```rust
use tracing_core::{Subscriber, Event, Metadata};
use tracing_subscriber::layer::Context;

struct DltForwarder { logger: FfiLogger }

impl<S> tracing_subscriber::Layer<S> for DltForwarder
where S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> {
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // extract fields
        let mut visitor = FieldVisitor::new(&self.logger);
        event.record(&mut visitor);
        self.logger.finish_event();
    }
}

struct FieldVisitor<'a> { logger: &'a FfiLogger }
impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.logger.push_i64(value);
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.logger.push_str(value);
    }
    // ... other record_*
}
```

### Benefits

* Idiomatic for Rust apps that already use `tracing`.
* No stringification and full typed preservation.

### POC instructions (Approach #2)

1. Implement same C ABI shim as Approach #1 (or reuse it).
2. Implement `DltForwarder` `Layer` in Rust which uses the FFI to push typed fields.
3. Register the layer with `tracing_subscriber::Registry::default().with(DltForwarder).init()`.
4. Use `tracing::info!` macros in Rust — fields are forwarded typed.

---

## Part F — Alternate approach: direct protocol / TCP

### Idea

* Implement the **DLT protocol** encoder directly in Rust and send DLT messages to `dlt-daemon` over Unix/TCP socket. This bypasses the C++ recorder entirely.

### Pros

* No C++ dependency; pure-Rust implementation.
* You control message construction and can tightly integrate with Rust ecosystems.

### Cons

* You must replicate DLT protocol correctness: headers, timestamps, session/ECU IDs, endian, type tags, optional serial header, fragmentation, socket reconnection, etc.
* This duplicates work already done in working C++ library.

### Sketch (what to implement)

* Rust module to build: StandardHeader, ExtendedHeader, typed argument encoders.
* Socket layer to send to daemon (Unix socket path e.g. `/tmp/dlt` or configured TCP address).

### When to choose this

* You want full Rust implementation, portability, or to avoid FFI/ABI concerns.

---

## Part G — Executable POC plan (both approaches)

I will outline the minimal code, folder layout, and run steps so you — or your CI — can build and demo quickly.

### POC1: C ABI shim + Rust FFI

**Files**

```
cpp/
  logging.h           // your existing logging interface
  logging.cpp         // existing
  logger_c_api.h
  logger_c_api.cpp
  CMakeLists.txt
rust_poc1/
  Cargo.toml
  src/ffi.rs
  src/lib.rs
  src/main.rs
```

**Build**

```bash
# Build C++ lib
cd cpp && mkdir build && cd build && cmake .. && make
# create a liblogger_c_api.a (or .so)

# Build Rust
cd ../../rust_poc1
# set env so rust can find the library
export LD_LIBRARY_PATH=../cpp/build
cargo run --release
```

**Test**

* Start `dlt-daemon` in background.
* Run Rust binary that writes a few typed messages.
* Start `dlt-viewer` or `dlt-receive` to confirm messages show typed fields.

### POC2: `tracing` subscriber + FFI

**Files**

```
rust_tracing_poc/
  Cargo.toml
  src/ffi.rs           // same C ABI bindings
  src/dlt_layer.rs     // tracing layer implementation
  src/main.rs          // demo using tracing::info!
```

**Build & Run**

```bash
# Build shared C++ shim as above
# Build rust_tracing_poc and run
cargo run --release
```

**Test**

* Use tracing macros from Rust and confirm typed values appear in DLT viewer.

---

## Part H — Example snippets (copy/paste friendly)

### C++ shim header (`logger_c_api.h`)

```cpp
#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct LoggerHandle LoggerHandle;
typedef struct LogStreamHandle LogStreamHandle;

LoggerHandle* logger_create(const char* app, const char* ctx);
void logger_destroy(LoggerHandle*);
LogStreamHandle* logger_begin(LoggerHandle*, int level);
void logger_push_str(LogStreamHandle*, const char* ptr, uint32_t len);
void logger_push_i32(LogStreamHandle*, int32_t v);
void logger_push_u32(LogStreamHandle*, uint32_t v);
void logger_push_i64(LogStreamHandle*, int64_t v);
void logger_push_f32(LogStreamHandle*, float v);
void logger_push_f64(LogStreamHandle*, double v);
void logger_push_raw(LogStreamHandle*, const uint8_t* ptr, uint32_t len);
void logger_finish(LogStreamHandle*);

#ifdef __cplusplus
}
#endif
```

### Rust FFI bindings (minimal `ffi.rs`)

```rust
#[repr(C)] pub struct LoggerHandle { _private: [u8; 0] }
#[repr(C)] pub struct LogStreamHandle { _private: [u8; 0] }

#[link(name = "logger_c_api")] extern "C" {
    fn logger_create(app: *const i8, ctx: *const i8) -> *mut LoggerHandle;
    fn logger_destroy(h: *mut LoggerHandle);
    fn logger_begin(h: *mut LoggerHandle, level: i32) -> *mut LogStreamHandle;
    fn logger_push_str(s: *mut LogStreamHandle, ptr: *const i8, len: u32);
    fn logger_push_i32(s: *mut LogStreamHandle, v: i32);
    fn logger_finish(s: *mut LogStreamHandle);
}

// Safe wrapper omitted here for brevity — see earlier message for full code.
```

### tracing-layer visitor (core methods)

```rust
impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.logger.push_i64(value);
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.logger.push_u64(value);
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.logger.push_u32(if value { 1 } else { 0 });
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.logger.push_str(value);
    }
}
```

---

### Preserving typed logging from Rust to DLT via C++ recorder

**Problem:** `log` crate stringifies arguments and loses type semantics; DLT requires typed arguments for correct decoding.

**Solution options:**

* **C ABI shim + Rust wrapper**: minimal change; Rust calls typed push functions; C++ handles DLT encoding.
* **`tracing` Subscriber → FFI**: integrate natively with Rust structured logging; forwarding typed fields through FFI.
* **Direct DLT in Rust**: implement DLT encoder in Rust and connect to daemon directly (more work).

**Recommendation:** Start with *C ABI shim* POC (fastest) and add `tracing` Layer once POC works.

**Next steps & timeline (example):**

1. Week 1: Implement C shim + Rust wrapper POC. Validate with dlt-viewer.
2. Week 2: Implement `tracing` Layer and integrate into services.
3. Week 3: Harden (thread-safety, buffering, reconnection) and measure performance.

---

## Appendix: caveats & notes

* Watch ABIs: `size_t`/`usize` and endianness; prefer fixed-width types (`int32_t`, `uint32_t`).
* Avoid C++ exceptions across FFI boundary — mark shim functions `noexcept` and catch exceptions inside shim.
* Thread-safety: ensure `Logger` and `Recorder` are thread-safe or protect with mutexes in shim.
* For `tracing` Layer, ensure that `visit` calls are fast and non-blocking — buffer or offload to a background thread if high-throughput.

---
