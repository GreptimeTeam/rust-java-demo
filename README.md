# RustJavaDemo

## Introduction

This is a demo project that shows the interop between Rust and Java, through the JNI. 

It demonstrates the following usages:

1. Assemble a single unified JAR that can run in multi platforms (aarch64 MacOS or x86 Linux etc), so that you don't need to manage lots of platform specific artifacts. And this is done in a convenient Github Action.
2. Basic function calls from Java to Rust defined native methods, and from Rust to the Java side.
3. Async method invocations in Rust, which are called from sync Rust functions that are provided for the JNI.
4. Logging practice that unifies Rust logs and Java logs.
5. Rust errors throw as `RuntimeException`s in Java.

All these features are very needed in any serious business projects. You can also use this demo as a template to start developing your own project.

## Build

Clone this repo, and simply run `mvn package` in the project root. 

The prerequisites are Java version 17 or above, and Rust toolchain.

## Example

You can find an example in `example/src/RustJavaDemoExample.java`. It fetches the web page content from the URL you provided as a cli argument. Once you build this project, you can run this example like this:

```bash
cd example/target
java -cp ./rust-java-demo-example.jar RustJavaDemoExample "http://www.greptime.com"
```

It will print the content of our website "http://www.greptime.com".
