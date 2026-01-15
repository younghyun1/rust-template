#![feature(const_type_name)]

use crate::setup_logger::setup_logger;

pub mod build_info;
pub mod setup_logger;

use mimalloc::MiMalloc;
use tracing::info;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // more accurately, tokio runtime start time - unfurl macro to get more accurate startup measurement
    // typically off by like four miliseconds
    let app_start_time = tokio::time::Instant::now();

    // these guards need to stay alive for the global logger to work
    let (_log_guard, _stdout_guard) = setup_logger().await;
    let _span_entered = tracing::info_span!(std::any::type_name_of_val(&main)).entered();
    info!(duration = ?app_start_time.elapsed(), "Logger initialized!");
}
