# Ferrum

> 一个简单、安全、现代化的 JavaScript/TypeScript 运行时

Ferrum 是一个受 Deno 启发的轻量级 JavaScript 和 TypeScript 运行时，使用 Rust 构建。它旨在为在浏览器外运行 JavaScript/TypeScript 提供一个安全高效的环境。

[English](README.md) | 简体中文

## 状态

**版本：** 0.1.0 (Alpha)

这是一个早期阶段的项目。核心功能已实现，但许多功能仍在开发中。详情请参阅[当前限制](#当前限制)。

## 特性

### 核心
- **安全性**：针对文件系统、网络和环境变量的显式权限模型
- **现代 ESM**：支持 ES2020 模块和 import maps
- **高性能**：基于 V8 JavaScript 引擎构建
- **单文件**：作为单个可执行文件分发

### 标准库
- **文件系统 API**：读取、写入、复制、重命名、目录操作
- **网络操作**：DNS 解析（HTTP/TCP 计划中）
- **定时器 API**：setTimeout、Promise（setInterval 开发中）
- **路径工具**：跨平台路径操作

### 开发体验
- **REPL**：支持多行输入的交互式 Shell
- **CLI**：丰富的命令行界面和权限标志
- **测试**：内置测试框架

## 安装

### 从源码构建
```bash
# 克隆仓库
git clone https://github.com/yourusername/ferrum.git
cd ferrum

# 构建并安装
cargo install --path .
```

### 预编译二进制文件
即将推出...

## 快速开始

### 运行脚本
```bash
ferrum run main.js
```

### REPL 模式
```bash
ferrum repl
> 1 + 1
2
> console.log("Hello")
Hello
```

### 使用权限
```bash
ferrum run --allow-read --allow-net script.js
```

## 使用示例

### Hello World
```javascript
// hello.js
console.log("Hello, Ferrum!");
```

运行：
```bash
ferrum run hello.js
```

### 文件操作
```javascript
// files.js
const data = "Hello, Ferrum!";
await Deno.writeTextFile("./output.txt", data);

const content = await Deno.readTextFile("./output.txt");
console.log(content);
```

运行：
```bash
ferrum run --allow-read --allow-write files.js
```

### DNS 查询
```javascript
// dns.js
// 注意：DNS 操作需要 --allow-net 权限
const ips = await Deno.resolveDns("example.com");
console.log(ips);
```

运行：
```bash
ferrum run --allow-net dns.js
```

## 权限系统

Ferrum 提供了安全的权限系统。默认情况下，脚本**无权访问**：

- 文件系统
- 网络
- 环境变量
- 子进程

### 授予权限

```bash
# 允许所有权限（请谨慎使用）
ferrum run --allow-all script.js

# 允许特定权限
ferrum run --allow-read --allow-net script.js

# 允许特定路径
ferrum run --allow-read-path=/tmp --allow-write-path=/tmp script.js

# 允许特定网络域名
ferrum run --allow-net-domain=github.com,api.github.com script.js

# 允许环境变量访问
ferrum run --allow-env script.js

# 允许子进程执行
ferrum run --allow-run script.js
```

## 架构

Ferrum 由多个关键组件构建而成：

```
┌─────────────────────────────────────────────────────────┐
│                      CLI 层                             │
│  (参数解析、权限管理、REPL)                              │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                   JavaScript 运行时                     │
│  (模块加载、执行、检查器)                                │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                      V8 引擎                            │
│  (JavaScript 执行、JIT 编译、垃圾回收)                   │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                    操作层 (Ops)                         │
│  (文件 I/O、网络、定时器等)                              │
└─────────────────────────────────────────────────────────┘
```

### 关键技术

- **Rust**：核心运行时实现
- **V8**：JavaScript 执行引擎
- **Tokio**：异步运行时
- **Clap**：命令行参数解析
- **Tracing**：结构化日志

## 项目结构

```
ferrum/
├── src/
│   ├── main.rs              # CLI 入口
│   ├── lib.rs               # 库入口
│   ├── cli.rs               # 命令行参数解析
│   ├── runtime.rs           # JavaScript 运行时设置
│   ├── module_loader.rs     # 模块解析和加载
│   ├── permissions.rs       # 权限系统
│   ├── repl.rs              # REPL 实现
│   ├── ops/                 # 原生操作
│   │   ├── mod.rs
│   │   ├── fs.rs           # 文件系统操作
│   │   ├── net.rs          # 网络操作
│   │   └── timers.rs       # 定时器操作
│   └── js/                  # 内置 JavaScript 文件
│       └── core.js         # 核心工具（待集成）
├── tests/                   # 集成测试
├── examples/                # 示例脚本
└── Cargo.toml
```

## 当前限制

这是一个 Alpha 版本，以下功能**尚未实现**：

### 网络
- **HTTP/HTTPS fetch** - API 结构已存在，需要集成 HTTP 客户端（reqwest/hyper）
- **WebSocket** - 已设计但未实现
- **TCP 连接** - 已设计但未实现

### 定时器
- **setInterval** - 定时器基础设施可用，但 callback 执行需要正确的 `FnMut` 处理

### 文件系统
- **文件监听** - 仅为占位符，需要集成 `notify` crate

### JavaScript 集成
- **V8-Rust 桥接** - 原生操作尚未暴露给 JavaScript
- **核心 JavaScript API** - `js/core.js` 引用了未实现的原生函数

### TypeScript
- **TypeScript 支持** - 计划在第四阶段
- **Source maps** - 计划在第三阶段

### 开发工具
- **测试运行器** - CLI 已存在，需要集成 JavaScript 测试框架
- **格式化工具** - 基础结构已存在，需要实现
- **调试器** - 检查器基础设施已存在，需要实现协议

## 开发路线图

### 第一阶段：核心运行时 (MVP) - 85% 完成
- [x] 基础 V8 集成
- [x] 模块加载 (ESM)
- [x] 权限系统
- [x] 文件系统操作（文件监听待完成）
- [x] 基础 REPL
- [x] DNS 解析

### 第二阶段：Web API - 20% 完成
- [ ] Fetch API (HTTP 客户端) - API 已设计，待实现
- [ ] WebSocket - API 已设计，待实现
- [ ] 文本编码/解码
- [ ] URL/URLSearchParams
- [ ] HTTP 服务器

### 第三阶段：开发工具 - 30% 完成
- [x] 测试运行器 CLI - 需要 JavaScript 集成
- [ ] 代码格式化工具 - 仅有结构
- [ ] Linter
- [ ] Source map 支持
- [ ] 调试器集成

### 第四阶段：高级特性
- [ ] TypeScript 编译器集成
- [ ] 包管理
- [ ] Worker 线程
- [ ] 插件系统
- [ ] 基于快照的启动

## 对比

| 特性 | Ferrum | Deno | Node.js |
|---------|--------|------|---------|
| 语言 | Rust | Rust | C++ |
| TypeScript | 计划中 | 原生支持 | 需要编译 |
| 安全性 | 权限系统 | 权限系统 | 无内置安全 |
| ESM | 默认支持 | 默认支持 | 可选 |
| 中心化包管理 | 无 | 无 | npm |
| 单文件分发 | 是 | 是 | 否 |

## 贡献

欢迎贡献！这是一个早期阶段的项目，有很多工作可以做。

### 优先领域

1. **HTTP 客户端集成** - 集成 reqwest 或 hyper 以实现 fetch API
2. **V8-Rust 桥接** - 将原生操作暴露给 JavaScript
3. **setInterval 修复** - 正确的 FnMut callback 处理
4. **文件监听** - 集成 notify crate
5. **测试** - 添加更多集成测试

指南请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)（即将推出）。

### 开发

```bash
# 克隆仓库
git clone https://github.com/yourusername/ferrum.git
cd ferrum

# 运行测试
cargo test

# 使用调试日志运行
RUST_LOG=debug cargo run -- run script.js

# 格式化代码
cargo fmt

# 代码检查
cargo clippy

# 构建发布版本
cargo build --release
```

## 许可证

MIT License - 详见 LICENSE 文件

## 致谢

- 灵感来自 [Deno](https://deno.land)
- 基于 [V8](https://v8.dev) 构建
- 使用 [Rust](https://www.rust-lang.org) 开发

## 名称

Ferrum 是拉丁语中的"铁"，代表作为运行时基础的强度和可靠性。
