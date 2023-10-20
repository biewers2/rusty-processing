//!
//! Code used by the Temporal worker.
//!
#![warn(missing_docs)]

use std::str::FromStr;
use std::sync::Arc;

use async_once::AsyncOnce;
use aws_sdk_s3 as s3;
use gethostname::gethostname;
use lazy_static::lazy_static;
use log::info;
use temporal_sdk::{sdk_client_options, Worker};
use temporal_sdk_core::{Client, CoreRuntime, init_worker, RetryClient};
use temporal_sdk_core_api::telemetry::TelemetryOptionsBuilder;
use temporal_sdk_core_api::worker::WorkerConfigBuilder;
use url::Url;

use services::config;

/// Temporal activity definitions.
///
pub mod activities;

/// I/O-related functionality.
///
pub(crate) mod io;

/// Utility functionality.
///
pub(crate) mod util;

const WORKER_BUILD_ID: &str = "rusty-mime-process-builder";
const TASK_QUEUE: &str = "rusty-mime-processing";
const NAMESPACE: &str = "default";

lazy_static! {
    static ref S3_CLIENT: AsyncOnce<s3::Client> = AsyncOnce::new(async {
        let config = aws_config::load_from_env().await;
        s3::Client::new(&config)
    });

    static ref REDIS: redis::Client = {
        redis::Client::open(REDIS_ADDRESS.as_ref()).unwrap()
    };

    static ref HOSTNAME: String = gethostname().to_string_lossy().to_string();

    static ref TEMPORAL_ADDRESS: String = format!(
        "http://{}:{}",
        config().get_or("TEMPORAL_HOST", "localhost"),
        config().get_or("TEMPORAL_PORT", "7233"),
    );

    static ref REDIS_ADDRESS: String = format!(
        "redis://{}:{}",
        config().get_or("REDIS_HOST", "localhost"),
        config().get_or("REDIS_PORT", "6379"),
    );
}

pub(crate) async fn s3_client() -> &'static s3::Client {
    S3_CLIENT.get().await
}

pub(crate) fn redis() -> &'static redis::Client {
    &REDIS
}

pub(crate) fn hostname() -> &'static str {
    HOSTNAME.as_str()
}

fn temporal_address() -> &'static str {
    TEMPORAL_ADDRESS.as_str()
}

/// Run the "dynamic" Temporal worker.
///
/// This worker is responsible for polling activity tasks on a generalized task queue.
///
/// It allows the activities to provide machine-specific task queues for future activities
/// in a workflow to use allowing for inter-activity filesystem access.
///
pub async fn run_dynamic_worker() -> anyhow::Result<()> {
    let client = connect_to_server().await?;
    let telemetry_options = TelemetryOptionsBuilder::default().build()?;
    let runtime = CoreRuntime::new_assume_tokio(telemetry_options)?;

    let worker_config = WorkerConfigBuilder::default()
        .worker_build_id(WORKER_BUILD_ID)
        .namespace(NAMESPACE)
        .task_queue(TASK_QUEUE)
        .build()?;

    info!("Creating static worker for task queue: {}", TASK_QUEUE);
    let core_worker = init_worker(&runtime, worker_config, client)?;
    let mut worker = Worker::new_from_core(Arc::new(core_worker), TASK_QUEUE);
    worker.register_activity("CreateWorkspace", activities::create_workspace);
    worker.run().await
}

/// Run the "sticky" Temporal worker.
///
/// This worker is responsible for polling activity tasks on a machine-specific task queue.
///
/// Activity tasks run on this worker will rely on the local filesystem having been operated
/// on by previous activity tasks.
///
pub async fn run_sticky_worker() -> anyhow::Result<()> {
    let client = connect_to_server().await?;
    let telemetry_options = TelemetryOptionsBuilder::default().build()?;
    let runtime = CoreRuntime::new_assume_tokio(telemetry_options)?;

    let task_queue = hostname();
    let worker_config = WorkerConfigBuilder::default()
        .worker_build_id(WORKER_BUILD_ID)
        .namespace(NAMESPACE)
        .task_queue(task_queue)
        .build()?;

    info!("Creating processing worker for task queue: {}", task_queue);
    let core_worker = init_worker(&runtime, worker_config, client)?;
    let mut worker = Worker::new_from_core(Arc::new(core_worker), task_queue);
    worker.register_activity("ProcessRustyFile", activities::process_rusty_file);
    worker.register_activity("RemoveWorkspace", activities::remove_workspace);
    worker.register_activity("Download", activities::download);
    worker.register_activity("Upload", activities::upload);
    worker.register_activity("Zip", activities::zip);
    worker.run().await
}

async fn connect_to_server() -> anyhow::Result<RetryClient<Client>> {
    let address = temporal_address();
    info!("Connecting to Temporal at {}", address);

    let server_options = sdk_client_options(Url::from_str(address)?).build()?;
    let client = server_options.connect(NAMESPACE, None, None).await?;
    info!("Connected!");

    Ok(client)

}
