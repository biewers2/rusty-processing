use std::str::FromStr;
use std::sync::Arc;
use log::info;

use temporal_sdk::{sdk_client_options, Worker};
use temporal_sdk_core::{CoreRuntime, init_worker};
use temporal_sdk_core_api::telemetry::TelemetryOptionsBuilder;
use temporal_sdk_core_api::worker::WorkerConfigBuilder;
use url::Url;
use services::config;

use temporal_worker::activities::process_rusty_file;

const WORKER_BUILD_ID: &str = "rusty-mime-process-builder";
const TASK_QUEUE: &str = "rusty-mime-processing";
const NAMESPACE: &str = "default";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Debug)?;

    let host = config().get_or("TEMPORAL_HOST", "localhost");
    let port = config().get_or("TEMPORAL_PORT", "7233");
    let url_str = format!("http://{}:{}", host, port);
    start_worker(url_str).await
}

async fn start_worker(address: impl AsRef<str>) -> anyhow::Result<()> {
    let addr = address.as_ref();
    info!("Connecting to Temporal at {}", addr);
    let server_options = sdk_client_options(Url::from_str(addr)?).build()?;
    let client = server_options.connect(NAMESPACE, None, None).await?;
    info!("Connected!");

    let telemetry_options = TelemetryOptionsBuilder::default().build()?;
    let runtime = CoreRuntime::new_assume_tokio(telemetry_options)?;

    let worker_config = WorkerConfigBuilder::default()
        .worker_build_id(WORKER_BUILD_ID)
        .namespace(NAMESPACE)
        .task_queue(TASK_QUEUE)
        .build()?;

    info!("Initializing worker");
    let core_worker = init_worker(&runtime, worker_config, client)?;
    let mut worker = Worker::new_from_core(Arc::new(core_worker), TASK_QUEUE);

    info!("Registering activities");
    // worker.register_wf("process_rusty_file", process_rusty_file);
    worker.register_activity("process_rusty_file", process_rusty_file);

    info!("Starting worker on task queue: {}", worker.task_queue());
    worker.run().await
}
