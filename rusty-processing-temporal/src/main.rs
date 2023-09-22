use std::str::FromStr;
use std::sync::Arc;

use temporal_sdk::{sdk_client_options, Worker};
use temporal_sdk_core::{CoreRuntime, init_worker};
use temporal_sdk_core_api::telemetry::TelemetryOptionsBuilder;
use temporal_sdk_core_api::worker::WorkerConfigBuilder;
use url::Url;

use rusty_processing_temporal::activities::process_rusty_file::{process_rusty_file, process_rusty_file_activity};

const WORKER_BUILD_ID: &str = "rusty-mime-process-builder";
const TASK_QUEUE: &str = "rusty-mime-process";
const NAMESPACE: &str = "default";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_worker().await
}

async fn start_worker() -> anyhow::Result<()> {
    let s3_uri = "s3://mime-processing-test/ubuntu-no.mbox";
    let output_s3_uri = "s3://mime-processing-test/ubuntu-no-small.mbox-output";
    let mimetype = "application/mbox";

    let (results, failures) = process_rusty_file(&s3_uri, &output_s3_uri, mimetype).await?;
    Ok(())

    // let server_options = sdk_client_options(Url::from_str("http://localhost:7233")?).build()?;
    //
    // let client = server_options.connect(NAMESPACE, None, None).await?;
    //
    // let telemetry_options = TelemetryOptionsBuilder::default().build()?;
    // let runtime = CoreRuntime::new_assume_tokio(telemetry_options)?;
    //
    // let worker_config = WorkerConfigBuilder::default()
    //     .worker_build_id(WORKER_BUILD_ID)
    //     .namespace(NAMESPACE)
    //     .task_queue(TASK_QUEUE)
    //     .build()?;
    //
    // let core_worker = init_worker(&runtime, worker_config, client)?;
    // let mut worker = Worker::new_from_core(Arc::new(core_worker), TASK_QUEUE);
    // worker.register_activity("process_rusty_file", process_rusty_file_activity);
    //
    // worker.run().await
}
