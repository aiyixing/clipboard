# Clipboard Monitor 开发文档

## 1. 项目概述

### 1.1 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 前端 | React + TypeScript | 18.x |
| 构建工具 | Vite | 5.x |
| 框架 | Tauri | 2.x |
| 后端 | Rust | 1.x |
| 剪贴板 | clipboard-rs | 0.3.x |

### 1.2 项目结构

```
clipboard/
├── src/                          # 前端代码 (React + TypeScript)
│   ├── components/
│   │   ├── TitleBar.tsx         # 标题栏组件
│   │   ├── StatusBar.tsx        # 状态栏组件
│   │   ├── Preview.tsx          # 剪贴板预览组件
│   │   └── HistoryList.tsx      # 历史记录列表组件
│   ├── App.tsx                   # 主应用组件
│   ├── main.tsx                  # 入口文件
│   └── styles.css                # 全局样式
│
├── src-tauri/                    # 后端代码 (Rust + Tauri)
│   ├── src/
│   │   ├── clipboard/
│   │   │   ├── mod.rs            # 剪贴板模块定义
│   │   │   ├── monitor.rs        # 剪贴板监听核心
│   │   │   └── storage.rs        # 数据存储
│   │   ├── hooks/
│   │   │   ├── mod.rs            # 钩子模块定义
│   │   │   └── runner.rs         # 钩子脚本执行器
│   │   ├── tray/
│   │   │   └── mod.rs            # 系统托盘管理
│   │   ├── commands.rs           # Tauri 命令接口
│   │   ├── lib.rs                # 库入口（公共类型和状态）
│   │   └── main.rs               # 应用入口
│   ├── Cargo.toml                # Rust 依赖配置
│   └── tauri.conf.json           # Tauri 配置
│
├── package.json                  # npm 依赖配置
├── tsconfig.json                 # TypeScript 配置
└── vite.config.ts                # Vite 配置
```

---

## 2. 环境准备

### 2.1 系统要求

- **操作系统**: Windows 10/11
- **Node.js**: >= 18.0.0
- **Rust**: >= 1.70.0
- **Visual Studio Build Tools**: 必需（Windows 平台）

### 2.2 安装步骤

#### 1. 安装 Node.js

从 [Node.js 官网](https://nodejs.org/) 下载并安装 LTS 版本。

#### 2. 安装 Rust

```powershell
# 使用 rustup 安装
winget install Rustlang.Rustup
```

或访问 [rustup.rs](https://rustup.rs/) 下载安装。

#### 3. 安装 Visual Studio Build Tools

从 [Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 下载并安装 "Desktop development with C++" 工作负载。

#### 4. 安装项目依赖

```powershell
# 安装 npm 依赖
npm install

# 安装 cargo 依赖（会自动安装）
cargo check
```

---

## 3. 开发指南

### 3.1 启动开发服务器

```powershell
# 同时启动前端和后端开发服务器
npm run tauri dev
```

或者：

```powershell
# 只启动前端开发服务器
npm run dev

# 只编译后端（调试模式）
cargo tauri build --debug
```

### 3.2 构建生产版本

```powershell
# 构建生产版本
npm run tauri build

# 或者只编译后端（生产模式）
cargo tauri build --release
```

构建输出位于：
- `src-tauri/target/release/clipboard-monitor.exe` - 可执行文件
- `src-tauri/target/release/bundle/` - 安装包

### 3.3 调试

#### 前端调试

在浏览器中打开前端开发服务器地址（通常是 `http://localhost:1420`），使用浏览器开发者工具调试。

#### 后端调试

设置环境变量 `RUST_LOG=debug` 来启用详细日志：

```powershell
$env:RUST_LOG = "debug"
.\src-tauri\target\debug\clipboard-monitor.exe
```

### 3.4 代码规范

#### Rust 代码

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量

```powershell
# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```

#### TypeScript 代码

- 使用 TypeScript 严格模式
- 遵循 React 函数组件最佳实践

---

## 4. 核心模块详解

### 4.1 剪贴板监听 (clipboard/monitor.rs)

#### 核心组件

1. **`ClipboardMonitorHandler`**
   - 实现 `clipboard_rs::ClipboardHandler` trait
   - `on_clipboard_change()` 方法在剪贴板变化时被调用

2. **`start_monitor()`**
   - 启动剪贴板监听器
   - 使用 `clipboard-rs` 的 `ClipboardWatcher`

#### 工作流程

```
1. Windows 剪贴板变化事件
   ↓
2. ClipboardWatcher 检测到变化
   ↓
3. 调用 on_clipboard_change()
   ↓
4. 读取剪贴板内容（文本/图片）
   ↓
5. 检查是否重复
   ↓
6. 非重复：存储数据 → 提取链接 → 触发钩子 → 通知前端
```

#### 关键代码片段

```rust
// 监听器实现
struct ClipboardMonitorHandler {
    app_handle: tauri::AppHandle,
    last_content: Arc<Mutex<Option<ClipboardContent>>>,
}

impl ClipboardHandler for ClipboardMonitorHandler {
    fn on_clipboard_change(&mut self) {
        // 读取剪贴板内容
        match read_clipboard_with_clipboard_rs() {
            Some(content) => {
                // 处理内容...
            }
            None => {
                // 无内容...
            }
        }
    }
}
```

### 4.2 数据存储 (clipboard/storage.rs)

#### 数据目录选择逻辑

程序会按以下顺序尝试数据目录：

1. **文档目录**: `C:\Users\{username}\Documents\ClipboardMonitor`
2. **用户主目录**: `C:\Users\{username}\ClipboardMonitor`
3. **当前工作目录**: `{project_dir}\data\ClipboardMonitor`

#### 存储格式

**Markdown 文件 (`clipboard_history.md`)**:

```markdown
## 2026-04-22 10:30:45
这是复制的文本内容...
---

## 2026-04-22 10:30:00
![图片](images/1776820350908.png)
---
```

**链接文件 (`links.txt`)**:

```text
https://example.com/page1
https://example.com/page2
```

### 4.3 Tauri 命令 (commands.rs)

#### 命令列表

| 命令 | 描述 |
|------|------|
| `get_clipboard_content` | 获取当前剪贴板内容 |
| `get_history` | 获取历史记录 |
| `clear_preview` | 清空预览区 |
| `toggle_monitoring` | 切换监听状态 |
| `get_monitoring_status` | 获取监听状态 |
| `open_data_directory` | 打开数据目录 |
| `copy_to_clipboard` | 复制文本到剪贴板 |

#### 前端调用示例

```typescript
import { invoke } from '@tauri-apps/api/core';

// 调用 Rust 命令
const result = await invoke('get_history', { count: 20 });
console.log(result);
```

### 4.4 系统托盘 (tray/mod.rs)

#### 托盘菜单项

1. **显示主窗口** - 显示主窗口
2. **暂停/恢复监听** - 切换监听状态
3. **退出** - 退出应用

---

## 5. 依赖说明

### 5.1 Rust 依赖 (Cargo.toml)

| 依赖 | 版本 | 用途 |
|------|------|------|
| tauri | 2.x | GUI 框架 |
| clipboard-rs | 0.3.x | 剪贴板操作（推荐） |
| clipboard-win | 5.x | Windows 剪贴板（备用） |
| arboard | 3.x | 跨平台剪贴板（备用） |
| chrono | 0.4.x | 日期时间处理 |
| tokio | 1.x | 异步运行时 |
| regex | 1.x | 正则表达式（链接提取） |
| dirs | 5.x | 系统目录操作 |
| log | 0.4.x | 日志接口 |
| env_logger | 0.11.x | 日志实现 |
| once_cell | 1.x | 延迟初始化 |
| lazy_static | 1.4.x | 静态变量 |
| open | 5.0.x | 打开文件/目录 |
| image | 0.25.x | 图像处理 |

### 5.2 npm 依赖 (package.json)

| 依赖 | 版本 | 用途 |
|------|------|------|
| react | 18.x | UI 框架 |
| react-dom | 18.x | DOM 渲染 |
| @tauri-apps/api | 2.x | Tauri API |
| vite | 5.x | 构建工具 |
| typescript | 5.x | 类型系统 |

---

## 6. 测试策略

### 6.1 自动测试功能

程序内置了自动测试功能，用于验证剪贴板监听是否正常工作。

**测试内容**:
- 每隔 10 秒复制不同的文本到剪贴板
- 格式: `自动测试文本 {count} - {time}`
- 共测试 5 次

**查看测试结果**:
1. 设置 `RUST_LOG=debug` 环境变量
2. 运行程序
3. 观察日志输出

### 6.2 手动测试步骤

1. **启动程序**
   ```powershell
   npm run tauri dev
   ```

2. **测试文本监听**
   - 复制任意文本到剪贴板
   - 检查主界面是否显示新内容
   - 检查 `clipboard_history.md` 是否有新记录

3. **测试图片监听**
   - 截图或复制图片
   - 检查 `images/` 目录是否有新图片
   - 检查 `clipboard_history.md` 是否有图片引用

4. **测试去重功能**
   - 复制相同文本两次
   - 确认只记录一次

5. **测试链接提取**
   - 复制包含 URL 的文本
   - 检查 `links.txt` 是否有提取的链接

---

## 7. 常见问题

### 7.1 编译问题

**问题**: `error: linking with link.exe failed`

**解决**: 确保已安装 Visual Studio Build Tools 并选择了 "Desktop development with C++" 工作负载。

**问题**: `icons/icon.ico not found`

**解决**: 运行 `cargo tauri icon` 生成图标，或手动创建图标文件。

### 7.2 运行时问题

**问题**: 剪贴板内容不更新

**解决**: 
- 确认使用的是 `clipboard-rs` 库（事件驱动）
- 检查日志是否有错误
- 尝试使用自动测试功能验证

**问题**: WebView2 错误: `请求的资源在使用中`

**解决**:
- 关闭所有 Tauri 实例
- 检查任务管理器，结束残留的 Tauri/WebView2 进程
- 重新启动程序

**问题**: 文件权限错误: `拒绝访问`

**解决**:
- 程序会自动尝试多个数据目录
- 最终会使用当前工作目录下的 `data/ClipboardMonitor`
- 检查日志确认实际使用的目录

### 7.3 依赖问题

**问题**: cargo 下载依赖慢

**解决**: 配置国内镜像源（如 rsproxy.cn）

```
# 在 %USERPROFILE%\.cargo\config.toml 中添加
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
```

---

## 8. 版本历史

### v0.1.0 (2026-04-22)

- 初始版本
- 基础剪贴板监听功能
- 文本和图片记录
- 系统托盘集成
- 主界面实现

---

## 9. 参考资源

- [Tauri 2.x 文档](https://v2.tauri.app/)
- [clipboard-rs GitHub](https://github.com/ChurchTao/clipboard-rs)
- [Rust 文档](https://doc.rust-lang.org/)
- [React 文档](https://react.dev/)
