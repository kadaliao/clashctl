# clashctl

**Simple-first TUI Clash Controller**

一个以"简单优先"为核心理念的终端 Clash 控制器，通过 External Controller API 管理 Clash 内核。

---

## ✨ 特色功能

- 🎯 **Simple-first 设计** - 默认简单，按需复杂，3 分钟上手
- ⚡ **非阻塞异步架构** - UI 永不卡顿，实时响应
- 🔄 **批量并发测速** - 一键测试所有节点，结果实时显示
- 🎨 **延迟评级配色** - Fast/Good/Slow 一目了然
- 🔐 **退出确认** - 防止误操作
- 📊 **双模式切换** - Simple/Expert 一键切换（Ctrl+E）
- 📝 **日志查看** - 实时查看 Clash 日志，支持过滤和搜索
- 👥 **节点分组** - 自定义节点组，快速管理常用节点
- 📈 **性能监控** - 实时查看流量统计和连接状态
- 🎨 **主题系统** - 4 种内置主题（Dark/Light/Dracula/Nord）
- 🔌 **连接管理** - 查看和管理活动连接
- ⚙️ **配置管理** - 导入导出配置，白名单/黑名单规则

---

## 🎉 开发状态: MVP 完成

所有核心功能已实现！可以日常使用。

### ✅ 已完成的功能

#### 核心页面（8/8）
- [x] **Home 页面** - 状态总览、模式切换、快速导航
- [x] **Routes 页面** - 双层级导航、节点切换、批量测速
- [x] **Rules 页面** - 规则查看、简单模式白名单/黑名单编辑
- [x] **Update 页面** - 订阅更新、状态提示
- [x] **Connections 页面** - 查看和管理活动连接
- [x] **Settings 页面** - 配置导入导出、收藏管理
- [x] **Logs 页面** - 日志查看、过滤和搜索
- [x] **Groups 页面** - 自定义节点分组管理
- [x] **Performance 页面** - 性能监控、流量统计

#### 交互功能
- [x] **节点管理** - 查看所有组和节点、手动切换
- [x] **批量测速** - 非阻塞异步测速、并发测试、结果缓存
- [x] **模式切换** - Rule/Global/Direct 循环切换
- [x] **订阅更新** - 一键更新所有 providers
- [x] **Simple/Expert 双模式** - Ctrl+E 切换
- [x] **节点收藏** - 快速标记和访问常用节点
- [x] **自定义规则** - 白名单/黑名单规则编辑
- [x] **连接管理** - 查看、关闭活动连接
- [x] **日志查看** - 日志过滤、搜索功能
- [x] **节点分组** - 创建、编辑、删除自定义分组
- [x] **性能监控** - 实时流量和连接统计

#### 用户体验
- [x] **状态反馈** - 操作提示、错误信息、测速结果
- [x] **退出确认** - 防止误操作
- [x] **自动刷新** - 每 5 秒自动刷新（可手动刷新）
- [x] **主题系统** - 4 种内置主题，Ctrl+T 切换
- [x] **配置持久化** - 所有设置自动保存

---

## 🚀 快速开始

### 1. 编译项目

```bash
# 克隆项目
git clone <repo-url>
cd clashctl

# 编译
cargo build --release
```

### 2. 运行

```bash
# 使用启动脚本（推荐）
./run.sh

# 或手动运行
./target/release/clashctl --api-url http://127.0.0.1:9090 --secret your_secret
```

### 3. 基础使用

```
1. 启动程序 → Home 页面
2. 按 g → Routes 页面
3. 选择线路 → 按 t 批量测速
4. 按 Enter → 展开节点列表
5. 选择最快节点 → 按 Enter 切换
6. 完成！享受丝滑体验 🚀
```

---

## 📖 使用指南

### 四个主要页面

#### 1. Home 页面

显示当前状态和快速操作菜单。

**快捷键**:
- `m` - 切换模式（Rule/Global/Direct）
- `g` - 跳转 Routes
- `n` - 跳转 Groups（节点分组）
- `l` - 跳转 Rules
- `c` - 跳转 Connections（连接管理）
- `o` - 跳转 Logs（日志查看）
- `p` - 跳转 Performance（性能监控）
- `u` - 跳转 Update
- `s` - 跳转 Settings
- `t` - 运行速度测试
- `r` - 刷新状态
- `Ctrl+T` - 切换主题
- `Ctrl+P` - 切换预设
- `q`/`Esc` - 退出（带确认）

---

#### 2. Routes 页面

管理代理组和节点。

**线路列表模式**:
- `↑↓` - 导航线路
- `Enter`/`→` - 展开节点列表
- `t` - 批量测试所有节点 ⭐
- `Ctrl+E` - 切换 Simple/Expert 模式
- `h` - 返回 Home
- `q` - 退出

**节点选择模式**:
- `↑↓` - 导航节点
- `Enter` - 切换到该节点
- `t` - 批量测试所有节点 ⭐
- `Esc`/`←` - 返回线路列表
- `h` - 返回 Home

**特色**:
- ⚡ **批量测速**: 按 `t` 一键测试所有节点，结果实时显示
- 🎨 **延迟评级**:
  - < 200ms: ⚡Fast (绿色)
  - 200-500ms: Good (黄色)
  - > 500ms: Slow (红色)
- 💾 **结果缓存**: 测速结果自动保存，无需重复测试

---

#### 3. Rules 页面

查看当前规则配置。

**快捷键**:
- `↑↓` - 滚动规则列表
- `Ctrl+E` - 切换 Simple/Expert 模式
- `h` - 返回 Home
- `g` - 跳转 Routes
- `q`/`Esc` - 退出

---

#### 4. Update 页面

更新订阅配置。

**快捷键**:
- `u` - 更新所有 providers
- `Ctrl+E` - 切换 Simple/Expert 模式
- `h` - 返回 Home
- `g` - 跳转 Routes
- `l` - 跳转 Rules
- `q`/`Esc` - 退出

---

### Simple vs Expert 模式

按 `Ctrl+E` 在所有页面中切换模式。

#### Simple 模式（默认）
- ✅ 简化显示，只显示常用选项
- ✅ 隐藏 GLOBAL 组和自动选择组
- ✅ 3 分钟上手
- ✅ 适合日常使用

#### Expert 模式
- ✅ 显示所有选项和详细信息
- ✅ 显示 GLOBAL 组和完整规则列表
- ✅ 适合高级用户

---

## 📋 使用场景

### 场景 1: 快速切换节点

```bash
./run.sh

# 1. 按 g 进入 Routes
# 2. 选择线路（如 HK-Group）
# 3. 按 Enter 展开节点
# 4. 用 ↑↓ 选择节点
# 5. 按 Enter 切换
# ✅ 完成！
```

---

### 场景 2: 找到最快节点

```bash
./run.sh

# 1. 按 g 进入 Routes
# 2. 选择线路（如 HK-Group）
# 3. 按 t 批量测速
# 4. 按 Enter 展开查看结果
# 5. 等待几秒，看到延迟评级:
#    HK-02 [156ms ⚡Fast]  ← 最快
#    HK-01 [201ms Good]
#    HK-03 [523ms Slow]
# 6. 选择 HK-02，按 Enter 切换
# ✅ 现在使用最快节点！
```

---

### 场景 3: 切换代理模式

```bash
./run.sh

# 在 Home 页面:
# 1. 按 m 切换模式
# 2. Rule → Global → Direct → Rule
# ✅ 模式已切换！
```

---

### 场景 4: 更新订阅

```bash
./run.sh

# 1. 按 u 进入 Update 页面
# 2. 按 u 更新所有订阅
# 3. 等待完成提示
# ✅ 订阅已更新！
```

---

## ⚙️ 配置

### CLI 参数

```bash
# 指定 API 地址
--api-url <URL>

# 指定 secret
--secret <SECRET>

# 帮助
--help

# 版本
--version
```

### 默认配置

- API 地址: `http://127.0.0.1:9090`
- Secret: 无（如果 Clash 有配置，需要提供）

### 配置优先级

```
CLI 参数 > 默认值
```

---

## 📚 文档

### 用户文档
- [USAGE.md](./USAGE.md) - 详细使用说明
- [FAQ.md](./docs/FAQ.md) - 故障排查指南

### 功能文档
- [批量测速](./docs/batch-speedtest-groups.md)
- [非阻塞测速](./docs/non-blocking-speedtest.md)
- [退出确认](./docs/quit-confirmation.md)
- [完成总结](./docs/COMPLETION_SUMMARY.md)

### 开发文档
- [功能总结](./FEATURES_SUMMARY.md)
- [实现总结](./IMPLEMENTATION_SUMMARY.md)
- [交互功能](./INTERACTIVE_FEATURES.md)

---

## 🛠️ 技术架构

### 核心技术栈

- **Rust 1.93.0** - 系统编程语言
- **ratatui 0.26** - TUI 框架
- **crossterm 0.27** - 终端控制
- **tokio 1** - 异步运行时
- **reqwest 0.11** - HTTP 客户端
- **clap 4** - CLI 参数解析
- **serde 1 + serde_json** - 序列化

### 架构亮点

1. **非阻塞异步架构**
   - tokio::spawn 后台任务
   - mpsc channel 结果通信
   - UI 永不卡顿

2. **双层级导航**
   - 代理组列表 → 节点列表
   - 信息层次清晰
   - 符合用户心智模型

3. **批量并发测速**
   - 所有节点并发测试
   - 结果实时显示
   - 自动评级配色

4. **Simple/Expert 双模式**
   - 默认简单，按需复杂
   - 全局一键切换
   - 同一界面，不同视角

---

## 📁 项目结构

```
clashctl/
├── Cargo.toml              # 项目配置
├── run.sh                  # 启动脚本
├── README.md              # 本文件
├── USAGE.md               # 使用指南
├── src/
│   ├── main.rs            # CLI 入口
│   ├── lib.rs             # 库入口
│   ├── app/
│   │   ├── state.rs       # 全局状态管理
│   │   └── mode.rs        # Simple/Expert 模式
│   ├── clash/
│   │   ├── client.rs      # Clash API 客户端
│   │   ├── types.rs       # API 数据结构
│   │   ├── models.rs      # 领域模型
│   │   └── mod.rs
│   └── ui/
│       ├── mod.rs         # TUI 主循环
│       └── pages/         # 4 个页面
│           ├── home.rs    # Home 页
│           ├── routes.rs  # Routes 页
│           ├── rules.rs   # Rules 页
│           └── update.rs  # Update 页
└── docs/                   # 详细文档
    ├── FAQ.md
    ├── COMPLETION_SUMMARY.md
    └── *.md
```

---

## 🔮 未来计划

### 短期（P0）
- [ ] 配置持久化到 `~/.config/clashctl/config.yaml`
- [ ] 规则编辑功能
- [ ] 基础单元测试

### 中期（P1）
- [ ] Preset 系统（Default/Work/Strict/Expert）
- [ ] Core 检测与安装
- [ ] 性能监控

### 长期（P2）
- [ ] 多语言支持
- [ ] 主题系统
- [ ] 插件系统

---

## ❓ 常见问题

### Q: 如何连接 Clash？

确保 Clash External Controller 已启用。查看 Clash 配置文件：

```yaml
external-controller: 127.0.0.1:9090
secret: "your_secret_here"
```

### Q: Routes 页面不显示线路？

1. 切换到 Expert 模式（Ctrl+E）查看是否有代理组
2. 检查 Clash 配置中是否有 `proxy-groups`
3. 按 `r` 刷新状态

### Q: 测速一直显示 Testing...？

等待 5 秒超时。如果节点无法连接，会显示失败。

### Q: 如何退出程序？

按 `q`、`Esc` 或 `Ctrl+C`，会显示确认对话框。按 `y` 确认退出。

**更多问题？** 查看 [FAQ.md](./docs/FAQ.md)

---

## 🤝 贡献

欢迎贡献代码！

1. Fork 项目
2. 创建功能分支
3. 提交代码
4. 发起 Pull Request

---

## 📄 License

MIT License

---

## 🙏 致谢

感谢以下开源项目：

- [ratatui](https://github.com/ratatui-org/ratatui) - 优秀的 TUI 框架
- [tokio](https://github.com/tokio-rs/tokio) - 强大的异步运行时
- [reqwest](https://github.com/seanmonstar/reqwest) - 易用的 HTTP 客户端
- [Clash](https://github.com/Dreamacro/clash) - 代理内核

---

## 📞 联系方式

- Issues: [GitHub Issues](https://github.com/yourusername/clashctl/issues)
- Discussions: [GitHub Discussions](https://github.com/yourusername/clashctl/discussions)

---

**立即开始使用 clashctl！** 🚀

```bash
./run.sh
按 g → 选择线路 → 按 t → 按 Enter → 看实时测速！
```

**享受丝滑的 TUI 体验！** ✨

---

**最后更新**: 2025-01-27
**版本**: v0.1.0
**状态**: MVP 完成 ✅
