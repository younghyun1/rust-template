use crate::setup_logger::setup_logger;

pub mod build_info;
pub mod setup_logger;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let (_log_guard, _stdout_guard) = setup_logger().await;
}
