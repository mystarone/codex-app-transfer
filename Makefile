# Codex App Transfer Backend (Tauri Shell Removed)
# 本地 Makefile
#   dev       - 运行后端开发服务器
#   build     - 构建后端可执行文件
#   clean     - 清理 target/

.PHONY: help dev build clean test

help:
	@echo "Targets:"
	@echo "  dev       Run backend development server"
	@echo "  build     Build backend executable"
	@echo "  test      Run tests"
	@echo "  clean     Remove target/"

dev:
	cargo run -p codex-app-transfer-server

build:
	cargo build -p codex-app-transfer-server --release

test:
	cargo test --workspace

clean:
	rm -rf target
