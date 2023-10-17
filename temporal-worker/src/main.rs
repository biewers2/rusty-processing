use tokio::try_join;
use temporal_worker::{run_dynamic_worker, run_sticky_worker};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;
    try_join!(
        run_dynamic_worker(),
        run_sticky_worker()
    )?;
    Ok(())
}