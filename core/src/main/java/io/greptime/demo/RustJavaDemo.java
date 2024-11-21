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

package io.greptime.demo;

import io.greptime.demo.utils.AsyncRegistry;
import io.greptime.demo.utils.Logger;
import io.questdb.jar.jni.JarJniLoader;
import java.util.concurrent.CompletableFuture;

/**
 * The main functionality of this class is implemented in Rust. Called from Java
 * via JNI. So it's important to load the Rust lib first. It's recommended to
 * call {@link #libInit()} like this:
 * 
 * <pre>
 * class YourMainClass {
 *     static {
 *         // Init the demo Rust lib, and create a runtime of size 8.
 *         RustJavaDemo.libInit(8);
 *     }
 *
 *     public static void main(String[] args) {
 *         // ...
 *     }
 * }
 * </pre>
 */
public class RustJavaDemo {

    private static final Logger LOGGER = Logger.getLogger(RustJavaDemo.class);

    static {
        JarJniLoader.loadLib(
                RustJavaDemo.class,
                "/io/greptime/demo/rust/libs",
                "demo");
    }

    /**
     * Load the demo rust lib and init it with a size of it's runtime. Setting
     * the `runtimeSize` to 0 will use the cpu's cores.
     * This method can be called multiple times, only the first call will take
     * effect.
     */
    public static native void libInit(int runtimeSize);

    /**
     * Hello to fetch the web content of the param `url`.
     */
    public CompletableFuture<String> hello(String url) {
        long futureId = nativeHello(url);
        return AsyncRegistry.take(futureId);
    }

    private native long nativeHello(String url);
}
