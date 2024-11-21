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

//! Borrowed some codes from https://questdb.io/blog/leveraging-rust-in-our-high-performance-java-database/

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::sync::Mutex;

use jni::errors::Result;
use jni::objects::{AutoLocal, GlobalRef, JMethodID, JValue};
use jni::signature::{Primitive, ReturnType};
use jni::JNIEnv;
use log::{Level, Log};

use crate::method_invoker::LOGGER;
use crate::{call_state, invoke_static_method, java_vm, unwrap_or_throw};

struct LogMethods {
    error: JMethodID,
    warn: JMethodID,
    info: JMethodID,
    debug: JMethodID,
    trace: JMethodID,
}

impl LogMethods {
    fn find_method(&self, level: Level) -> JMethodID {
        match level {
            Level::Error => self.error,
            Level::Warn => self.warn,
            Level::Info => self.info,
            Level::Debug => self.debug,
            Level::Trace => self.trace,
        }
    }
}

/// [CallState] is used to bridge the loggers between Rust and Java.
/// It initializes and stores the Java logger objects, and finds the Java methods to call.
pub(crate) struct CallState {
    /// Map a Rust module name to a Java `Logger` object.
    loggers: Mutex<HashMap<String, GlobalRef>>,
    methods: LogMethods,
}

impl CallState {
    pub(crate) fn try_new(env: &mut JNIEnv) -> Result<Self> {
        let class_name = "io/greptime/demo/utils/Logger";
        let methods = LogMethods {
            error: env.get_method_id(class_name, "error", "(Ljava/lang/String;)V")?,
            warn: env.get_method_id(class_name, "warn", "(Ljava/lang/String;)V")?,
            info: env.get_method_id(class_name, "info", "(Ljava/lang/String;)V")?,
            debug: env.get_method_id(class_name, "debug", "(Ljava/lang/String;)V")?,
            trace: env.get_method_id(class_name, "trace", "(Ljava/lang/String;)V")?,
        };
        Ok(Self {
            loggers: Mutex::new(HashMap::new()),
            methods,
        })
    }

    fn with_logger<F, R>(&self, env: &mut JNIEnv, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut JNIEnv, &GlobalRef) -> R,
    {
        let logger = {
            let mut loggers = self.loggers.lock().unwrap();
            match loggers.entry(name.to_string()) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(self.init_logger(env, name)?),
            }
            .clone()
        };
        Ok(f(env, &logger))
    }

    fn init_logger(&self, env: &mut JNIEnv, name: &str) -> Result<GlobalRef> {
        let name = JValue::from(&env.new_string(name)?).as_jni();
        let logger = invoke_static_method!(env, LOGGER, &[name])?;
        env.new_global_ref(logger.l()?).map_err(Into::into)
    }
}

/// Custom logger impl, that delegates log messages to Java loggers.
pub(crate) struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Discard JNI log messages to avoid infinite loops.
        !metadata.target().starts_with("jni")
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let env = &mut java_vm()
            .attach_current_thread_permanently()
            .expect("unable to attach current thread to JavaVM");
        let call_state = call_state();

        let msg = unwrap_or_throw!(env, format_msg(record));

        let r = call_state.with_logger(env, record.target(), |env, logger| {
            let r = env.new_string(msg).and_then(|msg| {
                // The log message object could stay in a special "registry" that is created by the JVM
                // for a long time here since the logging call is invoked in a static runtime
                // (for more details: https://stackoverflow.com/a/70946713/876147), so we need to explicitly
                // make it "deletable" or "gc-able".
                let msg = AutoLocal::new(msg, env);

                let method_id = call_state.methods.find_method(record.level());
                let msg = JValue::from(&msg).as_jni();
                unsafe {
                    env.call_method_unchecked(
                        logger,
                        method_id,
                        ReturnType::Primitive(Primitive::Void),
                        &[msg],
                    )
                }
            });
            let _ = unwrap_or_throw!(env, r);
        });
        unwrap_or_throw!(env, r)
    }

    fn flush(&self) {
        // Since the logging is delegated to Java loggers, flushing here is not necessary.
    }
}

/// Format the log message as "{file}:{line} - {args}".
fn format_msg(record: &log::Record) -> std::result::Result<String, fmt::Error> {
    let mut msg = String::new();

    if let Some(file) = record.file() {
        write!(msg, "{}", file)?;
    }

    if let Some(line) = record.line() {
        write!(msg, ":{}", line)?;
    }

    write!(msg, " - ")?;
    msg.write_fmt(*record.args())?;

    Ok(msg)
}

#[allow(unused)]
pub use log::{debug, error, info, trace, warn};
