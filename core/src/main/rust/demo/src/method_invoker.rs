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

#[cfg(test)]
mod test;

use std::ffi::c_void;
use std::sync::OnceLock;

use jni::errors::Result;
use jni::objects::{GlobalRef, JStaticMethodID, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use jni::sys::{jint, jvalue};
use jni::{JNIEnv, JavaVM};

use crate::JNI_VERSION;

pub(crate) static LOGGER: OnceLock<StaticMethodInvoker> = OnceLock::new();
pub(crate) static ASYNC_REGISTRY_REGISTER: OnceLock<StaticMethodInvoker> = OnceLock::new();
pub(crate) static ASYNC_REGISTRY_GET_FUTURE: OnceLock<StaticMethodInvoker> = OnceLock::new();

/// A struct that holds a the static method of the Java side, to be invoked later.
///
/// Why not directly use the more convenient `JNIEnv::call_static_method`?
///
/// Because for one, caching a method ID is faster than looking it up every time.
///
/// And two, we have found that the ad-hoc method invocations can fail in some case:
/// when calling inside from a separate tokio runtime, it may just freeze up,
/// not panicking nor returning any errors. This behavior could happen in a Springboot
/// project. It is weird and we don't exactly know why. Our best guess is that when
/// calling inside from the tokio runtime, the classloader is different than the one
/// that Springboot uses (the one that knows our class), and that causes the method
/// lookup to fail.
///
/// In a JNI native method, the classloader to use is the same as the caller in Java
/// side. However, the threads that are created by tokio runtime is undetermined when,
/// thus the classloader is also not determined, usually the system classloader.
///
/// There's no way to control when a tokio runtime thread is created, nor to say set
/// the classloader. Finally we came up with this solution, caching all the methods
/// that are to be invoked in the tokio runtime, guaranteed they can be found.
pub(crate) struct StaticMethodInvoker {
    class: GlobalRef,
    method_id: JStaticMethodID,
    ret: ReturnType,
}

impl StaticMethodInvoker {
    fn try_new(
        env: &mut JNIEnv,
        class_name: &str,
        method_name: &str,
        sig: &str,
        ret: ReturnType,
    ) -> Result<Self> {
        let class = env.find_class(class_name)?;
        let class = env.new_global_ref(class)?;
        let method_id = env.get_static_method_id(class_name, method_name, sig)?;
        Ok(Self {
            class,
            method_id,
            ret,
        })
    }

    pub(crate) unsafe fn invoke<'local>(
        &self,
        env: &mut JNIEnv<'local>,
        args: &[jvalue],
    ) -> Result<JValueOwned<'local>> {
        env.call_static_method_unchecked(&self.class, self.method_id, self.ret.clone(), args)
    }
}

#[macro_export]
macro_rules! invoke_static_method {
    ($env: ident, $invoker: ident, $args: expr) => {{
        let invoker = $invoker.get().unwrap_or_else(|| {
            panic!(
                "StaticMethodInvoker '{}' must to be initialized in 'JNI_OnLoad'!",
                stringify!($invoker)
            );
        });
        unsafe { invoker.invoke($env, $args) }
    }};
}

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    let env = &mut unsafe { vm.get_env(JNI_VERSION) }
        .unwrap_or_else(|e| panic!("Cannot get JNIEnv in 'JNI_OnLoad'? Error: {e:?}"));

    init(env).unwrap_or_else(|e| {
        panic!("Failed to initialize JNI_OnLoad: {e}");
    });

    JNI_VERSION.into()
}

fn init(env: &mut JNIEnv) -> Result<()> {
    LOGGER.get_or_try_init(|| {
        StaticMethodInvoker::try_new(
            env,
            "io/greptime/demo/utils/Logger",
            "getLogger",
            "(Ljava/lang/String;)Lio/greptime/demo/utils/Logger;",
            ReturnType::Object,
        )
    })?;
    ASYNC_REGISTRY_REGISTER.get_or_try_init(|| {
        StaticMethodInvoker::try_new(
            env,
            "io/greptime/demo/utils/AsyncRegistry",
            "register",
            "()J",
            ReturnType::Primitive(Primitive::Long),
        )
    })?;
    ASYNC_REGISTRY_GET_FUTURE.get_or_try_init(|| {
        StaticMethodInvoker::try_new(
            env,
            "io/greptime/demo/utils/AsyncRegistry",
            "get",
            "(J)Ljava/util/concurrent/CompletableFuture;",
            ReturnType::Object,
        )
    })?;
    Ok(())
}
