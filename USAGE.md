# clashctl 使用指南

## 快速开始

### 场景 1: 本地有 Clash 在运行,无需认证

```bash
cargo run
```

### 场景 2: 本地有 Clash 在运行,需要 secret 认证

如果看到 "Health: Error" 和 "401 Unauthorized",说明需要提供 secret:

```bash
# 方法 1: 命令行参数
cargo run -- --secret "your_secret_here"

# 方法 2: 环境变量
export CLASH_SECRET="your_secret_here"
cargo run
```

### 场景 3: Clash 在其他端口或地址

```bash
cargo run -- --api-url "http://127.0.0.1:9090" --secret "your_secret"
```

## 获取 Clash Secret

### 方法 1: 查看 Clash 配置文件

```bash
# Clash Verge / Clash for Windows
cat ~/.config/clash/config.yaml | grep secret

# 或者
grep "secret:" ~/.config/clash/config.yaml
```

### 方法 2: 从 Clash GUI 查看

- **Clash Verge**: 设置 → 外部控制 → Secret
- **Clash for Windows**: Settings → External Controller → Secret

## 常见问题

### Q: 看到 "Health: Error" 怎么办?

根据错误提示:

**401 Unauthorized** → 需要提供 secret
```bash
cargo run -- --secret YOUR_SECRET
```

**Connection refused** → Clash 未运行或端口不对
- 检查 Clash 是否在运行
- 确认 Clash 的 External Controller 端口(通常是 9090)

### Q: 找不到 secret 怎么办?

如果配置文件中没有 secret,可以自己设置:

1. 编辑 `~/.config/clash/config.yaml`
2. 添加或修改:
   ```yaml
   external-controller: 127.0.0.1:9090
   secret: "your_custom_secret_here"
   ```
3. 重启 Clash
4. 使用你设置的 secret 运行 clashctl

### Q: 如何测试连接?

使用测试模式:

```bash
cargo run -- --test --secret YOUR_SECRET
```

成功输出示例:
```
✓ Connected successfully!
✓ Configuration:
  Mode: Rule
  HTTP Port: 7890
  SOCKS Port: 7891
✓ Found 5 proxy groups:
  - Auto (Selector)
  - HK (Selector)
  ...
```

## 订阅更新

Update 页面支持两类订阅来源：

- Clash 配置中的 `proxy-providers`
- Mihomo Party 的 `profile.yaml` + `profiles/<id>.yaml`

快捷键：
- `Enter` - 更新当前订阅
- `u` - 更新所有订阅
- `r` - 刷新订阅列表

可通过环境变量覆盖订阅配置位置：

- `CLASH_CONFIG_PATH` - 指定 Clash 配置文件路径
- `CLASH_PARTY_DIR` - 指向 Mihomo Party 目录或其 `profile.yaml`

## 快捷键（常用）

- `q` / `Esc` - 返回/退出
- `r` - 刷新状态
- `Ctrl+C` - 强制退出
- `g` - Routes 页面
- `m` - 切换模式
- `u` - Update 页面
