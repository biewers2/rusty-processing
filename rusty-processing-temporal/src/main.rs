use std::str::FromStr;
use std::sync::Arc;


use temporal_sdk::{sdk_client_options, Worker};
use temporal_sdk_core::{CoreRuntime, init_worker};
use temporal_sdk_core_api::telemetry::TelemetryOptionsBuilder;
use temporal_sdk_core_api::worker::WorkerConfigBuilder;

use url::Url;
use rusty_processing_temporal::activities::create_workspace::create_workspace;
use rusty_processing_temporal::activities::destroy_workspace::destroy_workspace;
use rusty_processing_temporal::activities::download::download;

use rusty_processing_temporal::activities::process_rusty_file::process_rusty_file;
use rusty_processing_temporal::activities::upload::upload;

const WORKER_BUILD_ID: &'static str = "rusty-mime-processing-builder";
const TASK_QUEUE: &'static str = "rusty-mime-processing";
const NAMESPACE: &'static str = "default";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_worker().await
}

async fn start_worker() -> anyhow::Result<()> {
    let server_options = sdk_client_options(Url::from_str("http://localhost:7233")?).build()?;

    let client = server_options.connect(NAMESPACE, None, None).await?;

    let telemetry_options = TelemetryOptionsBuilder::default().build()?;
    let runtime = CoreRuntime::new_assume_tokio(telemetry_options)?;

    let worker_config = WorkerConfigBuilder::default()
        .worker_build_id(WORKER_BUILD_ID)
        .namespace(NAMESPACE)
        .task_queue(TASK_QUEUE)
        .build()?;

    let core_worker = init_worker(&runtime, worker_config, client)?;
    let mut worker = Worker::new_from_core(Arc::new(core_worker), TASK_QUEUE);
    worker.register_activity("create_rusty_workspace", create_workspace);
    worker.register_activity("destroy_rusty_workspace", destroy_workspace);
    worker.register_activity("download_rusty_file", download);
    worker.register_activity("process_rusty_file", process_rusty_file);
    worker.register_activity("upload_rusty_file", upload);

    worker.run().await
}
