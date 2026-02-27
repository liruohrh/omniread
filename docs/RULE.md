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

## 示例

```javascript
// 提取书籍信息
@js:(function() {
    return {
        title: text($("h1")),
        author: text($(".author")),
        cover: attr($("img.cover"), "src")
    };
})()

// 提取章节列表
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

## 数据源结构

```json
{
  "id": "example",
  "name": "示例站",
  "base_url": "https://example.com",
  "content_type": "novel",
  "search": {
    "url": "/search?q={keyword}",
    "list": ".result-item",
    "title": "@css:.title",
    "url_pattern": "@css:a@href"
  },
  "book_detail": {
    "title": "@css:h1",
    "author": "@css:.author",
    "cover": "@css:img.cover@src"
  },
  "chapter_list": {
    "list": ".chapter-item",
    "title": "@css:.title",
    "url_pattern": "@css:a@href"
  },
  "content": {
    "title": "@css:h1",
    "content": "@css:#content"
  }
}
```
