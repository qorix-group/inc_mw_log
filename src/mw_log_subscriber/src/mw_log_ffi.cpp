/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#include "score/mw/log/configuration/configuration.h"
#include "score/mw/log/log_level.h"
#include "score/mw/log/logger.h"
#include "score/mw/log/logging.h"

#include <iostream>

namespace score {
namespace mw {
namespace log {

extern "C" {

struct FfiValueData {
  union {
    int32_t i32_val;
    uint32_t u32_val;
    int64_t i64_val;
    uint64_t u64_val;
    double f64_val;
    bool bool_val;
    const char *str_ptr;
  };
};

struct FfiValue {
  uint8_t tag; // matches Rust's `tag`
  FfiValueData data;
};

void mw_log_send_record(const Logger *logger, uint8_t level,
                        const FfiValue *values, size_t len) {
  using score::mw::log::LogLevel;
  LogLevel lvl = static_cast<LogLevel>(level);

  score::mw::log::LogStream stream =
      (lvl == LogLevel::kError)   ? logger->LogError()
      : (lvl == LogLevel::kWarn)  ? logger->LogWarn()
      : (lvl == LogLevel::kInfo)  ? logger->LogInfo()
      : (lvl == LogLevel::kDebug) ? logger->LogDebug()
                                  : logger->LogVerbose();

  // 2. Append values
  for (size_t i = 0; i < len; i++) {
    const FfiValue &v = values[i];
    switch (v.tag) {
    case 0: // I32
      stream << v.data.i32_val;
      break;
    case 1: // U32
      stream << v.data.u32_val;
      break;
    case 2: // I64
      stream << v.data.i64_val;
      break;
    case 3: // U64
      stream << v.data.u64_val;
      break;
    case 4: // F64
      stream << v.data.f64_val;
      break;
    case 5: // Bool
      stream << (v.data.u32_val != 0 ? "true" : "false");
      break;
    case 6: // Str
      if (v.data.str_ptr) {
        stream << std::string(v.data.str_ptr);
      }
      break;
    default:
      stream << "<unknown>";
      break;
    }
    if (i + 1 < len) {
      stream << " "; // separator
    }
  }
}

struct LogValue {
  enum Kind { INT, UINT, FLOAT, BOOL, STR } kind;
  union {
    int64_t i;
    uint64_t u;
    double f;
    bool b;
    const char *s;
  } data;
};

struct LogRecord {
  uint8_t level;
  const char *target;
  const LogValue *args;
  size_t len;
};

Logger *mw_log_create_logger(const char *context) {
  return &CreateLogger(context);
}

bool mw_log_is_log_level_enabled_internal(const Logger *logger, uint8_t level) {
  return logger->IsLogEnabled(GetLogLevelFromU8(level));
}

uint8_t mw_log_logger_level_internal(const Logger *logger) {
  // TODO: This is adapter code, as there seems to be no way to get log level
  // for Logger
  if (logger->IsLogEnabled(LogLevel::kInfo)) {
    // Between Verbose, Debug, Info
    if (logger->IsLogEnabled(LogLevel::kDebug)) {
      if (logger->IsLogEnabled(LogLevel::kVerbose)) {
        return static_cast<uint8_t>(LogLevel::kVerbose);
      }

      return static_cast<uint8_t>(LogLevel::kDebug);
    }

    return static_cast<uint8_t>(LogLevel::kInfo);
  } else {
    // Lower half: Warn, Error, Fatal
    if (logger->IsLogEnabled(LogLevel::kError)) {
      if (logger->IsLogEnabled(LogLevel::kWarn)) {
        return static_cast<uint8_t>(LogLevel::kWarn);
      }

      return static_cast<uint8_t>(LogLevel::kError);
    }

    if (logger->IsLogEnabled(LogLevel::kFatal)) {
      return static_cast<uint8_t>(LogLevel::kFatal);
    }
  }

  // fallback
  return static_cast<uint8_t>(LogLevel::kOff);
}

} // extern "C"

} // namespace log
} // namespace mw
} // namespace score
