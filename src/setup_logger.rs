use std::path::Path;

use tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::build_info::{PROJECT_NAME, PROJECT_VERSION};

pub async fn setup_logger() -> (
    tracing_appender::non_blocking::WorkerGuard,
    tracing_appender::non_blocking::WorkerGuard,
) {
    let app_start_time = chrono::Utc::now();
    // 로그 파일 및 디렉토리
    let log_dir: &Path = Path::new("./logs");

    // 없으면 디렉토리 생성
    if !log_dir.exists()
        && let Err(e) = tokio::fs::create_dir_all(log_dir).await
    {
        eprintln!("Failed to create log directory './logs': {e}");
        std::process::exit(1);
    }

    // tracing 파일 로거 구성 (비동기 논블로킹)
    // 파일 자동 생성
    let file_appender = tracing_appender::rolling::never(
        "./logs",
        format!(
            "{}_{}_{}.log",
            PROJECT_NAME,
            PROJECT_VERSION,
            app_start_time.format("%Y%m%d_%H%M%S")
        ),
    );

    // 별도의 워커 스레드에서 로거를 실행하여 로깅이 작업 스레드 방해하지 않도록 설정
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 파일로 로깅할 때 JSON으로 구조적 로깅이 되도록, 그리고 터미널 아웃풋 캐릭터가 들어가지 않도록 설정
    let file_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

    // tracing stdout 로거 구성
    let (non_blocking_stdout, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());

    // 워커 스레드에서 로깅 구성
    let stdout_layer = fmt::layer()
        .pretty()
        .with_writer(non_blocking_stdout)
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    // 로거 초기화
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    (guard, stdout_guard)
}
