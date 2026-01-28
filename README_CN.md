# clashctl

一个基于 External Controller API 的 Clash TUI 控制器。

## 功能
- 终端内管理代理组与节点
- 批量测速，异步 UI 不阻塞（Work 预设会隐藏）
- Simple/Expert 双模式快速切换
- 订阅更新（proxy-providers、Mihomo Party）
- 查看连接与日志（日志为模拟数据，实时需 WebSocket）

## 依赖
- Rust（用于编译）
- 运行中的 Clash 内核，并开启 External Controller

## 快速开始

```bash
cargo build --release
./run.sh

# 或直接运行
./target/release/clashctl --api-url http://127.0.0.1:9090 --secret your_secret
```

## 常用快捷键
- `g` Routes，`m` 模式切换（Rule/Global/Direct）
- `t` 批量测速（Routes）
- `Enter` 切换节点
- `q`/`Esc` 退出（带确认）

## 配置
- 默认 API：`http://127.0.0.1:9090`
- CLI 参数：`--api-url`、`--secret`、`--help`、`--version`
- Update 页面订阅来源：
  - Clash 配置 `proxy-providers`
  - Mihomo Party `profile.yaml` + `profiles/<id>.yaml`
- 可用环境变量覆盖：`CLASH_CONFIG_PATH`、`CLASH_PARTY_DIR`
- 优先级：CLI 参数 > 默认值

## 文档
- `USAGE.md`

## 许可证
MIT
