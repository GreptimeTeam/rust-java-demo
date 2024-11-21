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

#![feature(once_cell_try)]

mod error;
mod logger;
mod method_invoker;

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

use error::{Error, JniSnafu, ReqwestSnafu, Result};
use futures::TryFutureExt;
use jni::objects::{AutoLocal, JClass, JObject, JString, JThrowable, JValue};
use jni::sys::jlong;
use jni::{sys, JNIEnv, JNIVersion, JavaVM};
use method_invoker::{ASYNC_REGISTRY_GET_FUTURE, ASYNC_REGISTRY_REGISTER};
use snafu::ResultExt;
use tokio::runtime::Runtime;

use crate::logger::{info, CallState, Logger};

const JNI_VERSION: JNIVersion = jni::JNIVersion::V1_8;

static INIT_LOCK: Mutex<bool> = Mutex::new(false);

const LOGGER: Logger = Logger;
static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

static CALL_STATE: OnceLock<CallState> = OnceLock::new();

thread_local! {
    static ENV: RefCell<Option<*mut jni::sys::JNIEnv>> = const { RefCell::new(None) };
}

fn runtime() -> &'static Runtime {
    RUNTIME
        .get()
        .expect("Runtime should have been initialized by calling the `libInit` first!")
}

fn java_vm() -> &'static JavaVM {
    JAVA_VM
        .get()
        .expect("JavaVM should have been initialized by calling the `libInit` first!")
}

fn call_state() -> &'static CallState {
    CALL_STATE
        .get()
        .expect("CallState should have been initialized by calling the `libInit` first!")
}

fn jni_env<'a>() -> JNIEnv<'a> {
    let env = ENV
        .with(|cell| *cell.borrow_mut())
        .expect("Not calling from inside the tokio runtime?");
    unsafe {
        JNIEnv::from_raw(env).unwrap_or_else(|e| panic!("Invalid 'JNIEnv' pointer? err: {:?}", e))
    }
}

#[no_mangle]
pub extern "system" fn Java_io_greptime_demo_RustJavaDemo_libInit(
    mut env: JNIEnv,
    _class: JClass,
    runtime_size: sys::jint,
) {
    let mut init = INIT_LOCK.lock().unwrap();
    if *init {
        return;
    }

    if runtime_size < 0 {
        throw_runtime_exception(&mut env, "`runtime_size` cannot be less than 0".to_string());
        return;
    }
    let runtime_size = if runtime_size == 0 {
        num_cpus::get()
    } else {
        runtime_size as usize
    };

    let java_vm = JAVA_VM.get_or_try_init(|| env.get_java_vm());
    let java_vm = unwrap_or_throw!(&mut env, java_vm);

    let runtime = RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(runtime_size)
            .on_thread_start(move || {
                ENV.with(|cell| {
                    let env =
                        unsafe { java_vm.attach_current_thread_as_daemon() }.unwrap_or_else(|e| {
                            panic!("Failed to attach tokio's threads to JVM, err: {e:?}")
                        });
                    *cell.borrow_mut() = Some(env.get_raw());
                })
            })
            .enable_all()
            .build()
    });
    unwrap_or_throw!(&mut env, runtime);

    let call_state = CALL_STATE.get_or_try_init(|| CallState::try_new(&mut env));
    unwrap_or_throw!(&mut env, call_state);

    GLOBAL_LOGGER.get_or_init(|| {
        log::set_logger(&LOGGER).expect("unable to set `Logger` as the global logger");
        log::set_max_level(log::LevelFilter::Trace);
        LOGGER
    });

    *init = true;

    info!(
        "RustJavaDemo Rust lib is initialized with runtime size {}",
        runtime_size
    );
}

#[no_mangle]
pub extern "system" fn Java_io_greptime_demo_RustJavaDemo_nativeHello<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass,
    url: JString<'a>,
) -> jlong {
    unwrap_or_throw!(&mut env, hello(&mut env, url), 0)
}

fn hello<'a>(env: &mut JNIEnv<'a>, url: JString<'a>) -> Result<jlong> {
    let url: String = env.get_string(&url).context(JniSnafu)?.into();

    let future_id = register(env).context(JniSnafu)?;
    runtime().spawn(async move {
        let result = reqwest::get(url)
            .and_then(|resp| resp.text())
            .await
            .context(ReqwestSnafu);

        let env = &mut jni_env();
        let result = result.and_then(|x| {
            env.new_string(x)
                .context(JniSnafu)
                .map(|x| AutoLocal::new(x.into(), env))
        });
        complete_future(future_id, result);
    });
    Ok(future_id)
}

// This future interaction between Java and Rust idea is borrow from OpenDAL, hats off to it!
fn complete_future<'a>(id: jlong, result: Result<AutoLocal<'a, JObject<'a>>>) {
    let env = &mut jni_env();
    let _ = env.with_local_frame(16, |env| -> jni::errors::Result<()> {
        let future = get_future(env, id)
            .unwrap_or_else(|e| panic!("Failed to get Java future by id '{}', error: {:?}", id, e));
        match result {
            Ok(result) => env
                .call_method(
                    future,
                    "complete",
                    "(Ljava/lang/Object;)Z",
                    &[JValue::Object(&result)],
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to complete Java future '{}' with result '{:?}', error: {:?}",
                        id, result, e
                    )
                }),
            Err(err) => {
                let ex = make_exception(env, &err).unwrap_or_else(|e| {
                    panic!(
                        "Failed to create Java exception for error '{:?}', error: {:?}",
                        err, e
                    )
                });
                env.call_method(
                    future,
                    "completeExceptionally",
                    "(Ljava/lang/Throwable;)Z",
                    &[JValue::Object(&ex)],
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to complete Java future '{}' with error '{:?}', error: {:?}",
                        id, err, e
                    )
                })
            }
        };
        Ok(())
    });
}

fn get_future<'local>(env: &mut JNIEnv<'local>, id: jlong) -> jni::errors::Result<JObject<'local>> {
    invoke_static_method!(env, ASYNC_REGISTRY_GET_FUTURE, &[JValue::Long(id).as_jni()])?.l()
}

fn make_exception<'local>(
    env: &mut JNIEnv<'local>,
    err: &Error,
) -> jni::errors::Result<JThrowable<'local>> {
    let exception = env.find_class("java/lang/RuntimeException")?;
    let err = env.new_string(format!("{:?}", err))?;
    env.new_object(exception, "(Ljava/lang/String;)V", &[JValue::Object(&err)])
        .map(JThrowable::from)
}

fn register(env: &mut JNIEnv) -> jni::errors::Result<jlong> {
    invoke_static_method!(env, ASYNC_REGISTRY_REGISTER, &[])?.j()
}

fn throw_runtime_exception(env: &mut JNIEnv, msg: String) {
    // There could be a pending exception that is thrown by calling into the Java side.
    // If we simply use the `msg` as the exception message, we would lose the original Java exception's details,
    // resulting in a vague "JavaException" error message.
    // It would be nice if https://github.com/jni-rs/jni-rs/pull/498/ is merged.
    let msg = if let Some(ex) = env.exception_occurred() {
        env.exception_clear();

        let class = env
            .get_object_class(&ex)
            .and_then(|x| env.call_method(x, "getName", "()Ljava/lang/String;", &[]))
            .and_then(|x| x.l())
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to get class name for exception: '{:?}', error: '{}'",
                    *ex, e
                )
            })
            .into();
        let class = env.get_string(&class).unwrap_or_else(|e| {
            panic!(
                "Failed to get string: '{:?}' from env, error: '{}'",
                *class, e
            )
        });

        let message = env
            .call_method(&ex, "getMessage", "()Ljava/lang/String;", &[])
            .and_then(|x| x.l())
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to get message for exception: '{:?}', error: '{}'",
                    *ex, e
                )
            })
            .into();
        let message = env.get_string(&message).unwrap_or_else(|e| {
            panic!(
                "Failed to get string: '{:?}' from env, error: '{}'",
                *message, e
            )
        });

        format!(
            "{}. Java exception occurred: {}: {}",
            msg,
            class.to_str(),
            message.to_str()
        )
    } else {
        msg
    };

    env.throw_new("java/lang/RuntimeException", &msg)
        .unwrap_or_else(|e| panic!("Failed to throw error '{msg}' as Java exception: {e:?}"));
}

/// Throws a Java exception if the result is an error.
/// The error will be formatted to string as the exception's message.
#[macro_export]
macro_rules! unwrap_or_throw {
    ($env:expr, $res:expr $(, $ret:expr)?) => {
        match $res {
            Ok(x) => x,
            Err(e) => {
                $crate::throw_runtime_exception($env, format!("{e:?}"));
                return $($ret)?;
            }
        }
    };
}
