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

use std::sync::OnceLock;

use jni::objects::JValue;
use jni::signature::{Primitive, ReturnType};
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

use super::StaticMethodInvoker;

fn jvm() -> &'static JavaVM {
    static JVM: OnceLock<JavaVM> = OnceLock::new();
    JVM.get_or_init(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .option("-Xcheck:jni")
            .build()
            .unwrap();
        JavaVM::new(jvm_args).unwrap()
    })
}

#[test]
fn test_static_method_invoker() {
    let mut env = jvm().attach_current_thread().unwrap();

    let invoker = StaticMethodInvoker::try_new(
        &mut env,
        "java/lang/Math",
        "abs",
        "(I)I",
        ReturnType::Primitive(Primitive::Int),
    )
    .unwrap();

    let result = unsafe { invoker.invoke(&mut env, &[JValue::Int(-10).as_jni()]) }
        .and_then(|x| x.i())
        .unwrap();
    assert_eq!(result, 10);
}
