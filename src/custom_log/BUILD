# *******************************************************************************
# Copyright (c) 2025 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************

load("@rules_rust//cargo:defs.bzl", "cargo_build_script")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")


rust_library(
    name = "custom_log",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "custom_log",
    edition = "2021",
    visibility = ["//visibility:public"],
    deps = [
        "//src/mw_log_subscriber",
        "//src/mw_log",
    ],
)
