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

use snafu::prelude::*;
use snafu::Location;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("JNI error: {:?} at {}", error, loc))]
    Jni {
        #[snafu(source)]
        error: jni::errors::Error,
        #[snafu(implicit)]
        loc: Location,
    },

    #[snafu(display("Reqwest error: {:?} at {}", error, loc))]
    Reqwest {
        #[snafu(source)]
        error: reqwest::Error,
        #[snafu(implicit)]
        loc: Location,
    },
}