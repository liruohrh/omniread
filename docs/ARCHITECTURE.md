# Architecture

记录 OmniRead 的架构设计分析与技术选型决策。

---

## 1. Headless WebView 渲染

### 需求

打开 URL，执行 JS（等待元素/条件渲染完成），获取渲染后的完整 HTML。支持取消（用户退出/超时）。

### 选型：Dart `flutter_inappwebview` HeadlessInAppWebView

Android 的 `android.webkit.WebView` 是平台 UI 组件，硬性约束：

- 必须在主线程（UI thread）创建和操作
- 需要 Android `Context`
- 内部依赖 `Looper` 消息循环

Rust 无法直接创建 Android WebView。如果要从 Rust 调用，必须通过 JNI 调 Java，在 Java 侧创建 WebView，再通过 JNI 回调传回 Rust。这等于重新实现 `flutter_inappwebview` 做的事情，没有实际收益。

**结论**：用 Dart `HeadlessInAppWebView`（`flutter_inappwebview` 包）。它是对系统 WebView 的封装，底层就是 Android WebView / iOS WKWebView。

### 取消/超时机制：Flutter 端统一 cancel

不把 timeout 传入 WebView 层，而是由调用方在 Flutter 端管理。

**理由**：

1. 用户退出和超时的底层操作相同（dispose WebView），统一为 `cancel()` 一个方法
2. 渲染器只管渲染，调用方决定超时策略，职责解耦
3. Dart `Future.timeout()` 成熟可靠，无需在 WebView 层重复实现
4. 一个机制覆盖两种场景：
   - 用户退出页面 → `widget.dispose()` → `renderer.cancel()`
   - 后台缓存超时 → `Future.timeout(duration)` → `renderer.cancel()`

### 实现

- `lib/src/services/headless_renderer.dart` — `HeadlessRenderer` 类
- `render(url, jsCode)` — 创建 HeadlessInAppWebView，加载 URL，`onLoadStop` 后执行 `callAsyncJavaScript`（支持 async），然后 `evaluateJavascript('document.documentElement.outerHTML')` 获取 HTML
- `cancel()` — 设置取消标志 + completeError + dispose WebView
- 竞态处理：每步检查 `_isCancelled` 和 `_completer!.isCompleted`

---

## 2. HTML 解析引擎：Rust + 嵌入式 JS 引擎

### 需求

拿到渲染后的 HTML，用用户定义的 JS 脚本进行解析（提取标题、章节列表、图片等），JS 中可以直接操作 DOM/解析对象。

### 候选方案对比

#### 方案 A：Dart 反射

- **不可行**。`dart:mirrors` 在 Flutter 中禁用，Dart 没有运行时反射。

#### 方案 B：Dart 嵌入 JS 引擎（如 `flutter_js`）

- 底层是 QuickJS，但 JS 和 Dart 的互操作能力弱
- 无法让 JS 直接操作 Dart 对象
- Dart 的 HTML 解析库（`html` 包）性能一般

#### 方案 C：Rust 嵌入 JS 引擎 + Rust HTML 解析器（推荐）

架构：

```
[Dart HeadlessWebView]
    |  render URL → HTML string
    v
[Rust via flutter_rust_bridge]
    |  接收 HTML string
    |  用 scraper / lol_html 解析 DOM
    |  用 JS 引擎执行用户的解析规则脚本
    |  JS 脚本里直接操作 Rust DOM 对象
    v
[返回结构化数据给 Dart]
```

**优势**：

1. Rust HTML 解析器（`scraper`, `lol_html`）性能远好于 Dart
2. JS 调用 Rust 对象是零拷贝 — 直接操作内存中的 Rust 结构体，无需序列化
3. 跨平台一致 — JS 引擎嵌在 Rust 里，Android/iOS/Desktop 行为完全一致

### JS 引擎选型

| 引擎 | 底层 | 优点 | 缺点 |
|------|------|------|------|
| **`boa_engine`** | 纯 Rust | 无 C 依赖，交叉编译简单；可注册 Rust 对象到 JS | 性能弱于 QuickJS；ES 规范支持不完整 |
| **`rquickjs`** | QuickJS (C) | 性能好，完整 ES2020；`#[derive]` 宏可将 Rust struct 暴露为 JS class | 有 C 依赖，交叉编译需配置 |
| **`deno_core`** | V8 | 性能最好 | 体积极大，对移动端不友好 |

**推荐 `rquickjs`**：性能与功能平衡，QuickJS 体积小适合移动端，`#[derive]` 宏让 JS 调 Rust 结构体接近零额外代码。

### JS 调 Rust 示例（rquickjs）

```rust
// Rust 侧定义
#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
struct HtmlDoc { /* scraper 解析后的 DOM */ }

#[rquickjs::methods]
impl HtmlDoc {
    fn query_selector(&self, sel: String) -> Vec<Element> { ... }
    fn text(&self, sel: String) -> String { ... }
}

// JS 脚本中直接写：
// let title = doc.query_selector("h1")[0].text();
// let items = doc.query_selector(".item").map(e => e.text());
```

### 不推荐的方案

| 方案 | 原因 |
|------|------|
| Dart 反射 | Flutter 禁用 `dart:mirrors`，不可行 |
| Dart 嵌 JS 引擎 | JS-Dart 互操作弱，无法让 JS 直接操作对象 |
| Rust 直接调 Android WebView | 必须 JNI 调 Java，等于重写 flutter_inappwebview，无收益 |
