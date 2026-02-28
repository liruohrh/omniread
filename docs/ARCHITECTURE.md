# Architecture

记录 OmniRead 的架构设计分析与技术选型决策。

---

## 1. Headless WebView 渲染

### 需求

打开 URL，执行 JS（等待元素/条件渲染完成），获取渲染后的完整 HTML。支持取消（用户退出/超时）。

### 选型：Dart `flutter_inappwebview` HeadlessInAppWebView

**版本要求**：`>=6.1.0`

| 版本 | iOS/macOS 支持情况 |
|------|-------------------|
| 6.0.x | macOS 仅支持 InAppBrowser，不支持 InAppWebView widget；iOS 存在 `callAsyncJavaScript` 崩溃问题 |
| 6.1.0+ | 添加 InAppWebView widget macOS 支持；修复多项 iOS/macOS 稳定性问题 |

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

- `lib/src/services/headless_renderer.dart` — `HtmlRenderer` 类
- `render(url, jsCode)` — 创建 HeadlessInAppWebView，加载 URL，`onLoadStop` 后执行 `callAsyncJavaScript`（支持 async），然后 `evaluateJavascript('document.documentElement.outerHTML')` 获取 HTML
- `cancel()` — 设置取消标志 + completeError + dispose WebView
- 竞态处理：每步检查 `_isCancelled` 和 `_completer!.isCompleted`

- 对于非法 js，android 仅输出 console，不会抛出异常，而 ios、macos 会抛出异常"A JavaScript exception occurred"。
    - "Uncaught SyntaxError: Unexpected token 'new'": 表示某个地方的new 前面有非法语法
        - error=3, 覆盖 error 无用，好像是用系统级 api
        - inappwebview consoleMessage api 只有 message 和 level，因此无法判断在哪一行。
    - 运行时错误都会抛出异常
- 只有新版本 ios、macos 支持原生实现的异步 JS，其他都是依靠监听机制。 
- inappwebview的异步js是作为函数体存在，因此如果需要返回值得 return，如果想返回一个 Promise，要么await，要么 return

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

**实际选用 `boa_engine`**：

- 纯 Rust 实现，无 C 依赖，交叉编译最简单
- ES 规范符合率已达 94%+，满足 HTML 解析脚本需求
- `scraper` crate 作为 HTML 解析器，性能优秀

### 实际实现架构

```
┌─────────────────────────────────────────────────────────────────┐
│                         Dart (Flutter)                          │
├─────────────────────────────────────────────────────────────────┤
│  1. HeadlessWebView 渲染 URL → HTML string                       │
│  2. 调用 Rust 解析 API（通过 flutter_rust_bridge）                 │
│     - 直接传 HTML + 解析脚本                                      │
│     - 或传 URL 让 Rust 自行请求（不需要 WebView 的场景）            │
│  3. 接收结构化数据（NovelInfo, ChapterInfo, ChapterContent）       │
└─────────────────────────────────────────────────────────────────┘
                              │ FFI
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Rust (rust_lib_omniread)                  │
├─────────────────────────────────────────────────────────────────┤
│  html_parser 模块：                                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ JsRuntime                                                   ││
│  │  ├── boa_engine::Context (JS 执行上下文)                     ││
│  │  ├── 注册全局 API: document, getText, getAttr, etc.          ││
│  │  ├── set_document(html) → 用 scraper 解析存入 thread_local   ││
│  │  └── execute(script) → 执行 JS 脚本，返回 JSON                ││
│  └─────────────────────────────────────────────────────────────┘│
│  rule 模块：                                                    ││
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 规则结构体定义：Source, BookDetail, ChapterList, Content      ││
│  │ 支持 YAML 和 JSON 格式的规则文件解析                          ││
│  │ 规则数组：支持多个规则的链式执行                              ││
│  └─────────────────────────────────────────────────────────────┘│
│  rule_parser 模块：                                             ││
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 规则解析器：执行规则数组，支持链式执行                        ││
│  │ 支持多种规则类型：CSS, JS, 正则表达式, JSON Path             ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  数据流：                                                        │
│  HTML string → scraper::Html → 规则解析 → JSON → Dart            │
└─────────────────────────────────────────────────────────────────┘
```

### 暴露给 JS 的 API

| 函数 | 签名 | 说明 |
|------|------|------|
| `$(selector)` | `(selector) → Element \| null` | CSS 选择器查询单个元素 |
| `$$(selector)` | `(selector) → Element[]` | CSS 选择器查询所有匹配元素 |
| `$(parent, selector)` | `(parent, selector) → Element \| null` | 在父元素内查询 |
| `$$(parent, selector)` | `(parent, selector) → Element[]` | 在父元素内查询所有 |
| `text(el)` | `(element) → string` | 获取元素的所有后代文本 |
| `ownText(el)` | `(element) → string` | 仅获取直接子文本节点 |
| `attr(el, name)` | `(element, name) → string \| null` | 获取元素属性值 |
| `html(el)` | `(element) → string` | 获取元素的 innerHTML |
| `hasClass(el, name)` | `(element, className) → boolean` | 检查元素是否有指定 class |
| `log(...)` | `(...args) → void` | 调试输出 |
| `setHtml(html)` | `(html) → void` | 动态设置当前文档 |
| `sharedGet(key)` | `(key) → string \| null` | 获取共享变量 |
| `sharedSet(key, value)` | `(key, value) → void` | 设置共享变量 |
| `webview(options)` | `(url \| options) → null` | 请求 WebView 渲染页面 |
| `setNextPage(url)` | `(url \| null) → void` | 设置下一页 URL（分页内容） |
| `base64Encode/Decode` | `(str) → string` | Base64 编解码 |
| `hexEncode/Decode` | `(str) → string` | Hex 编解码 |
| `md5(str)` | `(str) → string` | MD5 哈希 |
| `sha256(str)` | `(str) → string` | SHA256 哈希 |

### 上下文对象（Context）

JS 脚本执行时可访问以下预注入的上下文对象：

| 对象 | 说明 |
|------|------|
| `book` | 当前书籍信息（id, title, author, cover, url, extra） |
| `source` | 数据源信息（id, name, base_url, headers） |
| `page` | 当前页面信息（index, total, url, next_url, prev_url） |
| `parseMode` | 解析模式（book_detail, chapter_list, content, book_and_chapters, all） |
| 用户变量 | 用户配置的变量直接作为全局变量（如 cookie, token） |

### 共享变量机制

共享变量允许在不同解析步骤之间传递数据：

```
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  解析书籍详情    │  →   │   解析章节列表   │  →   │   解析章节内容   │
│  sharedSet(     │      │   sharedGet(    │      │   sharedGet(    │
│    "bookId",    │      │     "bookId"    │      │     "bookId"    │
│    "123"        │      │   ) → "123"     │      │   ) → "123"     │
│  )              │      │                 │      │                 │
└─────────────────┘      └─────────────────┘      └─────────────────┘
```

### WebView API

`webview()` 是 JS 中的 API 函数（不是规则语法前缀），用于请求 Dart 层的 HeadlessWebView 渲染需要 JavaScript 执行的页面：

```javascript
// 简单形式
webview("https://example.com/dynamic")

// 完整形式
webview({
    url: "https://example.com/spa",
    js: "document.querySelector('.load-more').click()",  // 可选
    waitFor: "#content"  // 可选，等待元素出现
})
```

**工作流程**：
1. JS 调用 `webview()` 返回 `null`
2. Rust 记录 WebView 请求，返回给 Dart
3. Dart 使用 HeadlessWebView 渲染 URL
4. 渲染后的 HTML 传回 Rust，继续执行解析

### 多页内容支持

支持以下场景：

| 场景 | 配置 | 说明 |
|------|------|------|
| 书籍+目录同页 | `book_and_chapters_same_page: true` | 一次请求获取书籍信息和章节列表 |
| 全部同页 | `all_in_one_page: true` | 动漫等场景，书籍/目录/内容在同一页 |
| 内容分页 | `content_paginated: true` | 章节内容分多页，JS 用 `setNextPage()` 指示 |
| 目录分页 | `chapter_list_paginated: true` | 章节列表分多页 |

### 性能特征（基准测试）

| 操作 | 耗时 | 说明 |
|------|------|------|
| JsRuntime 创建 | ~215 µs | 包含所有 API 注册 |
| HTML 解析（简单） | ~6 µs | 小型 HTML |
| HTML 解析（中等） | ~43 µs | 中等复杂度 HTML |
| 完整工作流 | ~360 µs | 创建 + 解析 + 执行脚本 |

**结论**：Runtime 创建是主要开销，实际 DOM 操作很快。单次解析完全满足需求。

### 并发与线程安全

**当前设计**：
- HTML 文档存储在 `thread_local!` 中
- 每次调用创建新的 `JsRuntime` 实例

**Dart 调用场景分析**：
- `flutter_rust_bridge` 默认在独立线程池执行 Rust 代码
- 每个 FFI 调用可能在不同线程
- `thread_local!` 确保同一线程内的隔离，不同线程自然隔离

**设计评估**：当前设计适合以下场景：
1. ✅ 串行解析（一个页面解析完再解析下一个）
2. ✅ 并发解析（多个页面同时解析，每个在不同线程）
3. ✅ 无需 Dart 侧额外同步

### 未来扩展预留

计划支持的额外 JS API（尚未实现）：

| API | 用途 |
|-----|------|
| `http.get(url)` | JS 脚本内发起 HTTP 请求 |
| `http.post(url, body)` | POST 请求 |
| `crypto.aes(data, key)` | AES 解密（部分网站加密内容） |

已实现的 API：
- ✅ `base64Encode/Decode` — Base64 编解码
- ✅ `hexEncode/Decode` — Hex 编解码
- ✅ `md5/sha256` — 哈希函数
- ✅ `sharedGet/Set` — 共享变量
- ✅ `webview()` — WebView 渲染请求
- ✅ `setNextPage()` — 多页内容支持

这些 API 可以复用当前的 `NativeFunction` 注册机制，无需架构调整。

### 不推荐的方案

| 方案 | 原因 |
|------|------|
| Dart 反射 | Flutter 禁用 `dart:mirrors`，不可行 |
| Dart 嵌 JS 引擎 | JS-Dart 互操作弱，无法让 JS 直接操作对象 |
| Rust 直接调 Android WebView | 必须 JNI 调 Java，等于重写 flutter_inappwebview，无收益 |
