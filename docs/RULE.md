# 规则 (Rule)

规则用于定义如何从网页提取内容。

## 选择器格式

| 前缀 | 说明 | 示例 |
|------|------|------|
| `@css:` | CSS 选择器 | `@css:.title` |
| `@css:...@attr` | CSS + 取属性 | `@css:img@src` |
| `@js:` | JavaScript | `@js:text($(".title"))` |
| `@re:` | 正则 (取第一个捕获组) | `@re:id=(\d+)` |
| `@re:...##...` | 正则替换 | `@re:\s+## ` |
| `@json:` | JSON Path 选择器 | `@json:$.title` |

## JS API

### 查询 (jQuery 风格)
```javascript
$(selector)           // 选择第一个元素
$(parent, selector)   // 在 parent 内选择
$$(selector)          // 选择所有元素
$$(parent, selector)  // 在 parent 内选择所有
```

### 元素操作
```javascript
text(el)              // 获取所有后代文本
ownText(el)           // 仅直接子文本节点
attr(el, name)        // 获取属性
html(el)              // innerHTML
hasClass(el, name)    // 检查 class
```

### 编解码
```javascript
base64Encode(str)     base64Decode(str)
hexEncode(str)        hexDecode(str)
```

### 加密
```javascript
md5(str)              // -> hex
sha256(str)           // -> hex
```

### 工具
```javascript
log(...)              // 打印调试
setHtml(html)         // 设置当前文档
```

### 共享变量
```javascript
sharedGet(key)        // 获取共享变量
sharedSet(key, value) // 设置共享变量 (跨解析步骤保持)
```

共享变量用于在不同解析阶段间传递数据，例如：
- 从书籍详情页解析出的 ID 可在章节列表页使用
- 从目录页获取的信息可在内容页使用

### WebView API
```javascript
// 请求 WebView 渲染页面
webview(url)                    // 简单形式
webview({
    url: "https://...",         // 要渲染的 URL
    js: "optional js code",     // 页面加载后执行的 JS (可选)
    waitFor: ".selector"        // 等待元素出现 (可选)
})
```

**注意**: `webview` 是 JS 中调用的 API 函数，不是规则语法前缀。它用于请求 Dart 层的 HeadlessWebView 渲染需要 JavaScript 执行的页面。调用后返回 `null`，实际 HTML 将在下次执行时由 Dart 层提供。

### 多页内容
```javascript
setNextPage(url)      // 设置下一页 URL (分页内容)
setNextPage(null)     // 表示没有更多页
```

## 上下文对象

JS 脚本执行时可访问以下上下文对象：

### book - 当前书籍信息
```javascript
book.id               // 书籍 ID
book.title            // 标题
book.author           // 作者
book.cover            // 封面 URL
book.url              // 详情页 URL
book.extra            // 扩展字段 (object)
```

### source - 数据源信息
```javascript
source.id             // 数据源 ID
source.name           // 数据源名称
source.base_url       // 基础 URL
source.headers        // 请求头 (包含用户设置的 cookie 等)
```

### page - 当前页面信息 (多页内容时)
```javascript
page.index            // 当前页索引 (0-based)
page.total            // 总页数 (如果已知)
page.url              // 当前页 URL
page.next_url         // 下一页 URL
page.prev_url         // 上一页 URL
```

### parseMode - 解析模式
```javascript
parseMode             // "book_detail" | "chapter_list" | "content" | 
                      // "book_and_chapters" | "all"
```

用于同一页面包含多种内容时区分解析目标。

### 用户变量
用户在 App 中设置的变量（如 cookie）会直接注入为全局变量：
```javascript
cookie                // 如果数据源定义了名为 "cookie" 的用户变量
token                 // 如果数据源定义了名为 "token" 的用户变量
```

## 示例

### 提取书籍信息
```javascript
@js:(function() {
    return {
        title: text($("h1")),
        author: text($(".author")),
        cover: attr($("img.cover"), "src")
    };
})()
```

### 提取章节列表
```javascript
@js:(function() {
    let items = $$(".chapter-item");
    let result = [];
    for (let i = 0; i < items.length; i++) {
        result.push({
            title: text($(items[i], ".title")),
            url: attr($(items[i], "a"), "href")
        });
    }
    return result;
})()
```

### 使用共享变量
```javascript
@js:(function() {
    // 在书籍详情页保存 ID
    let bookId = attr($("#book"), "data-id");
    sharedSet("bookId", bookId);
    // ...
})()

// 在章节列表页使用
@js:(function() {
    let bookId = sharedGet("bookId");
    // 使用 bookId 构建请求...
})()
```

### JSON 数据解析
```javascript
// 从 JSON 数据中提取书籍信息
@json:$.novel.title
@json:$.novel.author
@json:$.novel.cover

// 提取嵌套数据
@json:$.novel.stats.rating

// 提取数组数据
@json:$.novel.tags[*]

// 提取章节信息
@json:$.novel.chapters.volumes[0].title
@json:$.novel.chapters.volumes[0].chapter_list[0].title
```

### 多页内容解析
```javascript
@js:(function() {
    let content = text($("#content"));
    let nextLink = $(".next-page:not(.disabled)");
    if (nextLink) {
        setNextPage(attr(nextLink, "href"));
    }
    return { content: content };
})()
```

### 需要 WebView 渲染的页面
```javascript
@js:(function() {
    // 如果页面需要 JS 渲染才能获取内容
    let dynamicContent = $("#dynamic-content");
    if (!dynamicContent || text(dynamicContent) === "") {
        // 请求 WebView 渲染
        return webview({
            url: source.base_url + "/ajax/content",
            waitFor: "#dynamic-content"
        });
    }
    return { content: text(dynamicContent) };
})()
```

### 规则数组（链式执行）

规则数组允许您按顺序执行多个规则，前一个规则的结果会作为后一个规则的输入。这对于复杂的解析场景非常有用，例如先提取一个区域，然后在该区域内进一步提取数据。

```yaml
# 规则数组示例 - 提取书名
book_detail:
  title: 
    - "@css:.novel-info h1"          # 先提取 h1 标签内容
    - "@re:([^-]+) - "              # 然后使用正则表达式提取书名
```

```javascript
// 规则数组的链式执行流程
// 1. 第一个规则: @css:.novel-info h1 → "遮天 - 辰东"
// 2. 第二个规则: @re:([^-]+) - " → "遮天"
// 最终结果: "遮天"
```

## 数据源结构

### YAML 格式（推荐）

```yaml
id: example
name: 示例站
base_url: https://example.com
content_type: novel

user_vars:
  - name: cookie
    label: Cookie
    var_type: cookie
    placeholder: 请输入登录后的 Cookie
    description: 用于访问 VIP 内容

headers:
  User-Agent: Mozilla/5.0...

search:
  url: /search?q={keyword}
  list: [".result-item"]
  id: ["@css:.item@data-id"]
  title: ["@css:.title"]
  url_pattern: ["@css:a@href"]

book_detail:
  title: ["@css:h1"]
  author: ["@css:.author"]
  cover: ["@css:img.cover@src"]

chapter_list:
  list: [".chapter-item"]
  id: ["@css:.item@data-id"]
  title: ["@css:.title"]
  url_pattern: ["@css:a@href"]
  next_page: ["@css:.pagination .next@href"]
  has_next_page: ["@css:.pagination .next:not(.disabled)"]

content:
  title: ["@css:h1"]
  content: ["@css:#content"]
  next_page: ["@css:.content-page .next@href"]
  has_next_page: ["@css:.content-page .next:not(.disabled)"]
```

### JSON 格式

```json
{
  "id": "example",
  "name": "示例站",
  "base_url": "https://example.com",
  "content_type": "novel",
  "user_vars": [
    {
      "name": "cookie",
      "label": "Cookie",
      "var_type": "cookie",
      "placeholder": "请输入登录后的 Cookie",
      "description": "用于访问 VIP 内容"
    }
  ],
  "headers": {
    "User-Agent": "Mozilla/5.0..."
  },
  "search": {
    "url": "/search?q={keyword}",
    "list": [".result-item"],
    "id": ["@css:.item@data-id"],
    "title": ["@css:.title"],
    "url_pattern": ["@css:a@href"]
  },
  "book_detail": {
    "title": ["@css:h1"],
    "author": ["@css:.author"],
    "cover": ["@css:img.cover@src"]
  },
  "chapter_list": {
    "list": [".chapter-item"],
    "id": ["@css:.item@data-id"],
    "title": ["@css:.title"],
    "url_pattern": ["@css:a@href"],
    "next_page": ["@css:.pagination .next@href"],
    "has_next_page": ["@css:.pagination .next:not(.disabled)"]
  },
  "content": {
    "title": ["@css:h1"],
    "content": ["@css:#content"],
    "next_page": ["@css:.content-page .next@href"],
    "has_next_page": ["@css:.content-page .next:not(.disabled)"]
  }
}
```

## 页面布局处理

### 场景 1: 书籍信息和目录在同一页面 (常见)
- **处理方式**：在 `book_detail` 和 `chapter_list` 中定义相应的规则，系统会在一次请求中同时解析两者。

### 场景 2: 书籍信息和目录在不同页面
- **处理方式**：在 `chapter_list` 中定义 `url` 规则，系统会先请求详情页获取书籍信息，再使用该 URL 请求目录页获取章节列表。

### 场景 3: 所有内容在同一页面 (如动漫单集页)
- **处理方式**：在 JS 规则中使用 `parseMode` 变量来区分不同的解析目标，根据当前的解析模式返回对应的数据。

### 场景 4: 内容分页
- **处理方式**：在 `content` 中定义 `next_page` 和 `has_next_page` 规则，系统会循环请求直到没有下一页，合并所有内容。

### 场景 5: 章节列表分页
- **处理方式**：在 `chapter_list` 中定义 `next_page` 规则，系统会循环请求所有分页，合并章节列表。

## 用户变量

用户变量允许普通用户在 App 设置中配置特定值（如 Cookie、Token），无需修改规则代码。

### 变量类型

| 类型 | 说明 | 输入组件 |
|------|------|----------|
| `text` | 普通文本 | 单行输入框 |
| `password` | 密码/密钥 | 密码输入框 |
| `cookie` | Cookie | 多行输入框 + Cookie 助手 |
| `number` | 数字 | 数字输入框 |
| `url` | URL | URL 输入框 |

### 定义示例

```json
{
  "user_vars": [
    {
      "name": "cookie",
      "label": "登录 Cookie",
      "var_type": "cookie",
      "required": false,
      "description": "登录后的 Cookie，用于访问 VIP 章节"
    },
    {
      "name": "quality",
      "label": "图片质量",
      "var_type": "text",
      "default": "high",
      "description": "可选: low, medium, high"
    }
  ]
}
```

### 在 JS 中使用

```javascript
@js:(function() {
    // 用户变量直接作为全局变量可用
    if (cookie) {
        // 用户已设置 cookie
        log("Using cookie:", cookie);
    }
    
    let q = quality || "high";  // 使用默认值
    // ...
})()
```
