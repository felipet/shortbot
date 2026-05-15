// Copyright 2026 Felipe Torres González
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Module with the logic for metrics.

use metrics_exporter_prometheus::{BuildError, PrometheusBuilder, PrometheusHandle};
use tokio::task::JoinHandle;

/// Shortbot doesn't generates a ton of metrics, so 30 seconds is a good interval.
const UPKEEP_INTERVAL: u64 = 30;

/// Configures the metrics exporter and yields a handle for an axum router.
pub fn setup_metrics() -> Result<PrometheusHandle, BuildError> {
    PrometheusBuilder::new()
        .with_recommended_naming(true)
        .install_recorder()
}

pub fn spawn_metrics_upkeep_task(metrics_handle: PrometheusHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            metrics_handle.run_upkeep();
            tokio::time::sleep(std::time::Duration::from_secs(UPKEEP_INTERVAL)).await;
        }
    })
}
