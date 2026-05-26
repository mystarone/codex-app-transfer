//! Codex App Transfer Backend Server
//! 独立运行的后端代理服务，不依赖 Tauri 桌面壳

use clap::Parser;
use codex_app_transfer_proxy::{build_router, resolver::StaticResolver};
use codex_app_transfer_registry::{
    load_raw_config,
    paths::config_file,
    schema::Config,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 监听地址
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// 监听端口
    #[arg(short, long, default_value = "18080")]
    port: u16,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    info!("Codex App Transfer Backend Server starting...");

    // 加载配置
    let config_path = match config_file() {
        Some(p) => p,
        None => {
            error!("Failed to determine config file path");
            std::process::exit(1);
        }
    };
    info!("Loading config from: {:?}", config_path);
    
    let raw_config = match load_raw_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let config: Config = match serde_json::from_value(raw_config) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to parse config: {}", e);
            std::process::exit(1);
        }
    };

    // 构建静态解析器
    let resolver = Arc::new(StaticResolver::new(
        config.gateway_api_key,
        config.providers,
        config.active_provider,
    ));

    // 构建 router
    let app = build_router(resolver);

    // 启动服务
    let addr = SocketAddr::new(
        args.host.parse().expect("Invalid host address"),
        args.port,
    );
    
    info!("Server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
