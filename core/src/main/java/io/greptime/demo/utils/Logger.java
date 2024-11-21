// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package io.greptime.demo.utils;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

import org.slf4j.LoggerFactory;

/**
 * A simple wrapper of slf4j logger, and more importantly, it's used in Rust
 * code for logging.
 */
public class Logger {

    private static final ConcurrentMap<String, Logger> LOGGERS = new ConcurrentHashMap<>();

    public static Logger getLogger(Class<?> clazz) {
        return getLogger(clazz.getName());
    }

    public static Logger getLogger(String name) {
        return LOGGERS.computeIfAbsent(name, x -> new Logger(LoggerFactory.getLogger(x)));
    }

    private final org.slf4j.Logger inner;

    private Logger(org.slf4j.Logger inner) {
        this.inner = inner;
    }

    public void error(String msg) {
        this.inner.error(msg);
    }

    public void error(String format, Object... arguments) {
        this.inner.error(format, arguments);
    }

    public void warn(String msg) {
        this.inner.warn(msg);
    }

    public void info(String msg) {
        this.inner.info(msg);
    }

    public void info(String format, Object... arguments) {
        this.inner.info(format, arguments);
    }

    public void debug(String msg) {
        this.inner.debug(msg);
    }

    public void debug(String format, Object... arguments) {
        this.inner.debug(format, arguments);
    }

    public void trace(String msg) {
        this.inner.trace(msg);
    }
}
