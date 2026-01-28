#!/bin/bash
# clashctl 启动脚本

# 设置 Rust 环境
export PATH="$HOME/.cargo/bin:$PATH"

# 运行 clashctl (带密钥)
cargo run -- --secret "lxyxyz" "$@"
