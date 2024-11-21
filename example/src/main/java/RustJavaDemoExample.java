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

import io.greptime.demo.RustJavaDemo;
import io.greptime.demo.utils.Logger;

public class RustJavaDemoExample {

    static final Logger LOGGER = Logger.getLogger(RustJavaDemoExample.class);

    private static final RustJavaDemo DEMO;

    static {
        RustJavaDemo.libInit(4);

        DEMO = new RustJavaDemo();
    }

    public static void main(String[] args) throws Exception {
        String url = args[0];
        LOGGER.info("Trying to fetch web content from url: '{}' by Rust", url);
        String content = DEMO.hello(url).get();
        LOGGER.info("Result: '{}'", content);
    }
}
