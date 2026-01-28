# clashctl - 简洁优先的 TUI Clash 控制器

<div align="center">

**简单模式 3 分钟上手 • 专家模式无限可能**

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Rust](https://img.shields.io/badge/rust-1.70+-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

[功能特性](#功能特性) • [快速开始](#快速开始) • [使用指南](#使用指南) • [快捷键](#快捷键速查) • [开发文档](#开发文档)

</div>

---

## 项目简介

**clashctl** 是一个以"简单优先"为核心理念的 Clash TUI 控制器。通过 Clash External Controller API 与 Clash 内核通信，为用户提供直观、高效的终端界面。

### 为什么选择 clashctl？

- 🎯 **简单模式优先** - 3 分钟上手，隐藏复杂度
- ⚡ **非阻塞异步** - UI 永不卡顿，流畅体验
- 🎨 **灵活的 Preset 系统** - 适应不同使用场景
- 💾 **配置持久化** - 自动保存，无需重复配置
- 🔄 **实时监控** - 连接状态、节点延迟一目了然
- 🛡️ **安全防护** - Strict 模式防止误操作

---

## 功能特性

### ✅ 已实现功能

#### 核心功能
- ✅ **Home 页面** - 状态概览、场景切换、节点测速
- ✅ **Routes 页面** - 线路/节点管理、批量测速、一键切换
- ✅ **Rules 页面** - 规则管理、白名单/黑名单、规则搜索
- ✅ **Connections 页面** - 实时连接监控、流量统计、连接管理
- ✅ **Settings 页面** - 配置导入/导出、设置管理

#### 高级功能
- ✅ **双模式系统** - Simple 模式（初学者）/ Expert 模式（高级用户）
- ✅ **Preset 系统** - Default / Work / Strict / Expert 四种预设
- ✅ **节点收藏** - 标记常用节点，快速识别
- ✅ **配置管理** - YAML 格式导入导出
- ✅ **自动刷新** - 状态、连接实时更新

#### 用户体验
- ✅ **退出确认** - 防止误操作退出
- ✅ **状态反馈** - 操作结果实时提示
- ✅ **智能导航** - 子页面返回 Home，Home 触发退出
- ✅ **上下文帮助** - 每个页面显示相关快捷键

---

## 快速开始

### 安装

#### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/yourusername/clashctl.git
cd clashctl

# 编译 Release 版本
cargo build --release

# 运行
./target/release/clashctl
```

#### 系统要求

- Rust 1.70+
- 运行中的 Clash 内核（mihomo、clash-verge 等）
- Clash External Controller API 已启用

### 首次运行

```bash
# 如果 Clash 运行在默认端口（9090），直接启动
./clashctl

# 自定义 API 地址和密钥
./clashctl --api-url http://127.0.0.1:9090 --secret your_secret

# 配置会自动保存到 ~/.config/clashctl/config.yaml
# 下次直接运行即可
```

### 3 分钟快速上手

```
1. 启动程序 → Home 页面
2. 按 g → 进入 Routes（线路管理）
3. 按 t → 批量测速所有节点
4. 按 Enter → 展开查看节点列表
5. 选择最快节点 → 按 Enter 切换
6. 完成！按 h 返回 Home
```

---

## 使用指南

### 页面导航

```
Home (主页)
 ├── g → Routes (线路管理)
 ├── l → Rules (规则管理)
 ├── c → Connections (连接监控)
 ├── s → Settings (设置)
 └── u → Update (订阅更新)
```

### Update 页面

用于更新订阅（Clash `proxy-providers` 与 Mihomo Party 订阅）。

**快捷键**：
```
Enter        更新当前订阅
u            更新所有订阅
r            刷新订阅列表
h            返回 Home
g            跳转 Routes
l            跳转 Rules
q / Esc      退出
```

### Preset 系统

按 `Ctrl+P` 切换不同的使用场景：

#### 1. Default - 日常使用
```
• 默认: Simple 模式
• 功能: 所有功能可用
• 适合: 普通用户日常使用
```

#### 2. Work - 工作环境
```
• 默认: Simple 模式
• 功能: 隐藏批量测速
• 适合: 办公环境，界面简洁
```

#### 3. Strict - 生产环境
```
• 默认: Simple 模式
• 功能: 需 Expert 模式才能切换节点
• 适合: 防止误操作的生产环境
```

#### 4. Expert - 高级用户
```
• 默认: Expert 模式
• 功能: 显示所有细节
• 适合: 熟练用户、调试场景
```

### Simple vs Expert 模式

按 `Ctrl+E` 在两种模式间切换：

| 特性 | Simple 模式 | Expert 模式 |
|------|------------|------------|
| 目标用户 | 初学者 | 高级用户 |
| Routes 页面 | 只显示线路 | 显示所有节点 |
| Rules 页面 | 白名单/黑名单 | 完整规则列表 |
| 信息密度 | 精简 | 详细 |
| 上手难度 | ⭐ | ⭐⭐⭐ |

---

## 快捷键速查

### 全局快捷键
```
q / Esc      返回/退出（子页面返回 Home，Home 页面退出）
Ctrl+C       强制退出（带确认）
h            快速返回 Home
Ctrl+E       切换 Simple/Expert 模式
Ctrl+P       切换 Preset
```

### Home 页面
```
m            切换场景 (Rule → Global → Direct)
g            跳转 Routes
l            跳转 Rules
c            跳转 Connections
s            跳转 Settings
t            测试当前节点速度
r            刷新状态
```

### Routes 页面
```
↑↓           导航
Enter / →    展开节点列表
*            收藏/取消收藏节点
t            批量测速
Esc / ←      返回线路列表
```

### Rules 页面（Simple 模式）
```
w            添加白名单（Always Proxy）
b            添加黑名单（Always Direct）
d            删除选中规则
↑↓           导航
```

### Connections 页面
```
↑↓           导航
d            关闭选中连接
a            关闭所有连接
r            手动刷新
```

### Settings 页面
```
e            导出配置到 YAML
i            导入配置从 YAML
y/n          确认/取消
```

完整快捷键参考：[docs/KEYBOARD_SHORTCUTS.md](docs/KEYBOARD_SHORTCUTS.md)

---

## 配置文件

### 配置位置

```
~/.config/clashctl/config.yaml
```

### 订阅配置发现

Update 页面会尝试从以下位置读取订阅信息：

- **Clash config**：`proxy-providers` 段落
- **Mihomo Party**：`profile.yaml` + `profiles/<id>.yaml`

可通过环境变量覆盖：

- `CLASH_CONFIG_PATH` - 指定 Clash 配置文件路径
- `CLASH_PARTY_DIR` - 指向 Mihomo Party 目录或其 `profile.yaml`

### 配置示例

```yaml
api_url: http://127.0.0.1:9090
secret: your_secret_here
default_mode: simple
current_preset: default

# 收藏的节点
favorite_nodes:
  - "HK 01"
  - "US 02"

# 白名单（Always Proxy）
whitelist:
  - google.com
  - github.com

# 黑名单（Always Direct）
blacklist:
  - example.com
  - local.dev
```

### 配置优先级

```
CLI 参数 > 配置文件 > 默认值
```

---

## 开发文档

### 项目结构

```
clashctl/
├── src/
│   ├── app/              # 应用逻辑
│   │   ├── mod.rs        # 模式定义
│   │   └── state.rs      # 状态管理
│   ├── clash/            # Clash API
│   │   ├── client.rs     # API 客户端
│   │   ├── types.rs      # 数据类型
│   │   └── models.rs     # 领域模型
│   ├── config/           # 配置管理
│   │   ├── mod.rs        # 配置读写
│   │   └── preset.rs     # Preset 系统
│   ├── ui/               # 用户界面
│   │   ├── mod.rs        # UI 主循环
│   │   └── pages/        # 页面组件
│   │       ├── home.rs
│   │       ├── routes.rs
│   │       ├── rules.rs
│   │       ├── connections.rs
│   │       └── settings.rs
│   └── main.rs           # 程序入口
└── docs/                 # 文档
```

### 技术栈

- **语言**: Rust 1.70+
- **TUI 框架**: ratatui 0.26
- **终端控制**: crossterm 0.27
- **异步运行时**: tokio 1.x
- **HTTP 客户端**: reqwest 0.11
- **序列化**: serde + serde_json + serde_yaml

### 开发指南

详细开发文档：
- [开发路线图](docs/DEVELOPMENT_ROADMAP.md)
- [实现总结](docs/IMPLEMENTATION_SUMMARY.md)
- [功能总结](docs/FEATURES_SUMMARY.md)
- [最终报告](docs/FINAL_PROJECT_REPORT.md)

---

## 性能指标

- **编译时间**: 4.6s (release)
- **二进制大小**: ~2MB (stripped)
- **内存占用**: 10-20MB
- **CPU 空闲**: <1%
- **启动时间**: <1s

---

## 常见问题

### Q: 无法连接到 Clash
**A**: 检查以下项：
1. Clash 是否正在运行
2. External Controller 是否启用（通常在 Clash 配置的 `external-controller: 127.0.0.1:9090`）
3. API 地址和密钥是否正确
4. 防火墙是否阻止连接

### Q: 为什么按 q 不退出？
**A**: 在子页面（Routes/Rules 等），`q` 返回 Home。只有在 Home 页面按 `q` 才触发退出确认。

### Q: 测速功能不可用
**A**: 某些 Preset（如 Work）会隐藏测速功能。按 `Ctrl+P` 切换到其他 Preset。

### Q: 无法切换节点
**A**: 如果使用 Strict Preset，需要先按 `Ctrl+E` 切换到 Expert 模式。

### Q: 配置文件损坏
**A**: 删除 `~/.config/clashctl/config.yaml`，重新启动程序会自动创建新配置。

更多问题：[docs/FAQ.md](docs/FAQ.md)

---

## 路线图

### ✅ 已完成（v0.1.0）

- Phase 0-6: 基础功能 + MVP
- Phase 8-9: 配置持久化 + Preset 系统
- Phase 11.1-11.2: 规则搜索 + 编辑
- Phase 12: 连接管理
- Phase 16: 节点收藏
- Phase 18: 配置导入/导出

### 🔄 计划中

- Phase 13: 日志查看（需要 WebSocket）
- Phase 14: 自定义节点分组
- Phase 15: 性能监控图表
- Phase 17: 主题系统

---

## 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 开发原则

- 保持"简单优先"理念
- Simple 模式功能 ≤ 7 个可操作项
- 新功能必须有对应快捷键
- 所有 I/O 操作必须异步非阻塞

---

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

---

## 致谢

- [ratatui](https://github.com/ratatui-org/ratatui) - 优秀的 TUI 框架
- [Clash](https://github.com/Dreamacro/clash) - 强大的代理工具
- [mihomo](https://github.com/MetaCubeX/mihomo) - Clash 内核分支

---

## 联系方式

- **Issues**: [GitHub Issues](https://github.com/yourusername/clashctl/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/clashctl/discussions)

---

<div align="center">

**⭐ 如果这个项目对你有帮助，请给个 Star！**

Made with ❤️ in Rust

</div>
