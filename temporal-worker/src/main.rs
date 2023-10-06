use tokio::io::BufReader;
use tokio::try_join;

use processing::io::async_read_to_stream;
use temporal_worker::activities::process_rusty_stream;

const WORKER_BUILD_ID: &str = "rusty-mime-process-builder";
const TASK_QUEUE: &str = "rusty-mime-process";
const NAMESPACE: &str = "default";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start_worker().await
}

async fn start_worker() -> anyhow::Result<()> {
    // let s3_uri = "s3://mime-processing-test/ubuntu-no-small.mbox";
    // let output_s3_uri = format!("{}.zip", s3_uri);

    // process_rusty_file(s3_uri, output_s3_uri, mimetype, true).await?;

    // let mimetype = "application/mbox";
    let mimetype = "application/zip";
    // let abs_path = "/home/biewers2/Repos/mime-processing/processing/processing/resources/mbox/ubuntu-no.mbox";
    // let abs_path = "/home/biewers2/Documents/dataset/output.zip";
    let abs_path = "/home/biewers2/Repos/mime-processing/rusty-processing/resources/zip/testzip.zip";

    let file = Box::new(BufReader::new(tokio::fs::File::open(abs_path).await?));
    let (stream, reading) = async_read_to_stream(file)?;
    let processing = process_rusty_stream(stream, mimetype, true);

    try_join!(reading, processing)?;
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
