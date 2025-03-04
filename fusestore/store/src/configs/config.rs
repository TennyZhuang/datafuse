// Copyright 2020-2021 The Datafuse Authors.
//
// SPDX-License-Identifier: Apache-2.0.

use structopt::StructOpt;
use structopt_toml::StructOptToml;

pub const FUSE_COMMIT_VERSION: &str = env!("FUSE_COMMIT_VERSION");

#[derive(Clone, Debug, serde::Deserialize, PartialEq, StructOpt, StructOptToml)]
pub struct Config {
    #[structopt(long, env = "FUSE_STORE_LOG_LEVEL", default_value = "INFO")]
    pub log_level: String,

    #[structopt(
        long,
        env = "FUSE_STORE_METRIC_API_ADDRESS",
        default_value = "127.0.0.1:7171"
    )]
    pub metric_api_address: String,

    #[structopt(
        long,
        env = "FUSE_STORE_FLIGHT_API_ADDRESS",
        default_value = "127.0.0.1:9191"
    )]
    pub flight_api_address: String,
}
