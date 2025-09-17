# Needed for Dash tool to check python dependency licenses.
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")
load("@rules_rust//rust:defs.bzl", "rust_binary")
filegroup(
    name = "cargo_lock",
    srcs = [
        "Cargo.lock",
    ],
    visibility = ["//visibility:public"],
)

rust_binary(
    name = "example",
    srcs = ["src/main.rs"],
    data = [
        "config/logging.json",
    ],
    edition = "2021",
    visibility = ["//visibility:public"],
    deps = [
        "//src/custom_log",
                "//src/mw_log_subscriber",
    ],
)
