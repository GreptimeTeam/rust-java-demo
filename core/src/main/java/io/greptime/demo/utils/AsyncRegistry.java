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

import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

// Inspired by OpenDAL's Java binding.
//
// [AsyncRegistry] is used to help executing async functions in Rust.
// It works like this:
//   1. The Rust side calls the `register` method to get a unique id for a [CompletableFuture],
//      which is created by [AsyncRegistry] and stored inside it.
//   2. The id is returned from Rust to Java.
//   3. The Java side use the id to `take` the [CompletableFuture] and compose with more actions.
//   4. When the async operation is done, the Rust side `get` the future to "complete" it, 
//      hence the future chain is initiated in Java side.
public class AsyncRegistry {

    private static final AtomicLong FUTURE_ID = new AtomicLong();

    private static final Map<Long, CompletableFuture<?>> FUTURE_REGISTRY = new ConcurrentHashMap<>();

    // Used internally by the Rust side.
    @SuppressWarnings("unused")
    private static long register() {
        long id = FUTURE_ID.incrementAndGet();
        FUTURE_REGISTRY.put(id, new CompletableFuture<>());
        return id;
    }

    private static CompletableFuture<?> get(long futureId) {
        return FUTURE_REGISTRY.get(futureId);
    }

    /**
     * Take the [CompletableFuture] associated with the future id.
     */
    @SuppressWarnings("unchecked")
    public static <T> CompletableFuture<T> take(long futureId) {
        CompletableFuture<?> f = get(futureId);
        if (f != null) {
            f.whenComplete((r, e) -> {
                FUTURE_REGISTRY.remove(futureId);
            });
        }
        return (CompletableFuture<T>) f;
    }
}
