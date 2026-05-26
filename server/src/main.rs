//! Codex App Transfer Backend Server
//! 独立运行的后端代理服务，不依赖 Tauri 桌面壳

use clap::Parser;
use codex_app_transfer_codex_integration::{apply_provider, ApplyConfig, CodexPaths};
use codex_app_transfer_proxy::{build_router, resolver::StaticResolver};
use codex_app_transfer_registry::{
    load_raw_config,
    model_context_policy::model_supports_1m,
    paths::config_file,
    schema::{Config, Provider},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 监听地址
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// 监听端口
    #[arg(short, long, default_value = "18080")]
    port: u16,

    /// 不自动应用配置到 Codex APP
    #[arg(long)]
    no_apply: bool,
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

    // 如果配置了 active provider 并且没有禁用 apply，则应用配置到 Codex APP
    if !args.no_apply {
        info!("==================================================");
        info!("Configuration Status Check");
        info!("==================================================");
        
        if let Some(active_provider_id) = &config.active_provider {
            info!("Active provider ID: {}", active_provider_id);
            
            if let Some(provider) = config.providers.iter().find(|p| &p.id == active_provider_id) {
                info!("Found provider: '{}'", provider.name);
                info!("Provider base URL: {}", provider.base_url);
                info!("Provider API format: {}", provider.api_format);
                
                if let Ok(paths) = CodexPaths::from_home_env() {
                    info!("Codex config path: {:?}", paths.config_toml);
                    info!("Codex auth path: {:?}", paths.auth_json);
                    
                    let default_model = provider.models.get("default").cloned().unwrap_or_default();
                    info!("Default model: {}", default_model);
                    
                    let caps_value = serde_json::to_value(&provider.model_capabilities).ok();
                    let supports_1m = model_supports_1m(&default_model, caps_value.as_ref());
                    info!("Supports 1M context: {}", supports_1m);
                    info!("Codex network access: {}", config.settings.codex_network_access);
                    
                    info!("\nApplying provider '{}' to Codex APP...", provider.name);
                    let apply_result = apply_provider(
                        &paths,
                        &ApplyConfig {
                            base_url: &format!("http://127.0.0.1:{}", args.port),
                            gateway_api_key: config.gateway_api_key.as_deref().unwrap_or(""),
                            supports_1m,
                            provider_name: &provider.name,
                            default_model: &default_model,
                            model_mappings: Some(&serde_json::to_value(&provider.models).unwrap()),
                            model_capabilities: Some(&serde_json::to_value(&provider.model_capabilities).unwrap()),
                            app_version: "v2.1.15",
                            codex_network_access: config.settings.codex_network_access,
                            codex_status_section_default_visible: true,
                        },
                    );
                    
                    match apply_result {
                        Ok(result) => {
                            info!("\n✅ SUCCESS: Config applied to Codex APP!");
                            info!("Config TOML updated: {}", result.config_toml_path);
                            info!("Auth JSON updated: {}", result.auth_json_path);
                            if result.snapshot_taken {
                                info!("📸 Original Codex config snapshot saved");
                            }
                            if result.model_context_window_set {
                                info!("Model context window configured for 1M tokens");
                            }
                            if result.model_catalog_json_set {
                                info!("Model catalog JSON set");
                            }
                            info!("\n🎉 Configuration is now active!");
                            info!("Restart Codex APP for changes to take effect.");
                        }
                        Err(e) => {
                            error!("\n❌ FAILED to apply config to Codex APP: {}", e);
                        }
                    }
                } else {
                    error!("Failed to get Codex paths");
                }
            } else {
                warn!("⚠️ Active provider '{}' not found in providers list!", active_provider_id);
                info!("Available providers:");
                for p in &config.providers {
                    info!("  - {} ({})", p.name, p.id);
                }
            }
        } else {
            info!("⚠️ No active provider configured in settings");
            info!("Please set an active provider in ~/.codex-app-transfer/config.json");
        }
        info!("==================================================\n");
    } else {
        info!("Skipping config apply (--no-apply flag used)");
    }

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
    info!("To disable auto-apply to Codex APP, use --no-apply flag");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
