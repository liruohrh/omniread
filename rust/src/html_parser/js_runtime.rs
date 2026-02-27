//! JavaScript runtime for HTML parsing using boa_engine

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use boa_engine::{
    js_string, object::ObjectInitializer, property::Attribute, Context, JsArgs, JsNativeError,
    JsResult, JsValue, NativeFunction, Source,
};
use md5::{Digest as Md5Digest, Md5};
use scraper::{Html, Node, Selector};
use sha2::{Digest as Sha2Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CURRENT_HTML: RefCell<Option<Html>> = RefCell::new(None);
}

/// JS runtime with DOM query APIs for HTML parsing.
pub struct JsRuntime {
    context: Context,
}

impl JsRuntime {
    pub fn new() -> Self {
        let mut context = Context::default();
        Self::register_api(&mut context);
        Self { context }
    }

    /// Execute JS script on HTML with optional variables.
    ///
    /// # JS APIs
    /// ## Query (jQuery-like)
    /// - `$(selector)` / `$(parent, selector)` -> Element | null
    /// - `$$(selector)` / `$$(parent, selector)` -> Element[]
    ///
    /// ## Element
    /// - `text(el)` -> string (all descendant text)
    /// - `ownText(el)` -> string (direct text nodes only)
    /// - `attr(el, name)` -> string | null
    /// - `html(el)` -> string (innerHTML)
    /// - `hasClass(el, name)` -> boolean
    ///
    /// ## Encoding
    /// - `base64Encode(str)` / `base64Decode(str)`
    /// - `hexEncode(str)` / `hexDecode(str)`
    ///
    /// ## Crypto
    /// - `md5(str)` / `sha256(str)` -> hex string
    ///
    /// ## Util
    /// - `log(...)` - print to stdout
    /// - `setHtml(html)` - set current document
    pub fn execute(
        &mut self,
        html: &str,
        script: &str,
        vars: Option<&HashMap<String, String>>,
    ) -> Result<String, String> {
        CURRENT_HTML.with(|cell| {
            *cell.borrow_mut() = Some(Html::parse_document(html));
        });

        if let Some(vars) = vars {
            for (name, value) in vars {
                self.context
                    .register_global_property(
                        js_string!(name.clone()),
                        js_string!(value.clone()),
                        Attribute::all(),
                    )
                    .ok();
            }
        }

        match self.context.eval(Source::from_bytes(script)) {
            Ok(value) => {
                let json = value
                    .to_json(&mut self.context)
                    .map_err(|e| format!("JSON error: {}", e))?;
                Ok(json.to_string())
            }
            Err(e) => Err(format!("JS error: {}", e)),
        }
    }

    fn register_api(ctx: &mut Context) {
        // Query
        ctx.register_global_builtin_callable(
            js_string!("$"),
            2,
            NativeFunction::from_fn_ptr(Self::js_select_one),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("$$"),
            2,
            NativeFunction::from_fn_ptr(Self::js_select_all),
        )
        .unwrap();

        // Element
        ctx.register_global_builtin_callable(
            js_string!("text"),
            1,
            NativeFunction::from_fn_ptr(Self::js_text),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("ownText"),
            1,
            NativeFunction::from_fn_ptr(Self::js_own_text),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("attr"),
            2,
            NativeFunction::from_fn_ptr(Self::js_attr),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("html"),
            1,
            NativeFunction::from_fn_ptr(Self::js_html),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("hasClass"),
            2,
            NativeFunction::from_fn_ptr(Self::js_has_class),
        )
        .unwrap();

        // Encoding
        ctx.register_global_builtin_callable(
            js_string!("base64Encode"),
            1,
            NativeFunction::from_fn_ptr(Self::js_base64_encode),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("base64Decode"),
            1,
            NativeFunction::from_fn_ptr(Self::js_base64_decode),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("hexEncode"),
            1,
            NativeFunction::from_fn_ptr(Self::js_hex_encode),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("hexDecode"),
            1,
            NativeFunction::from_fn_ptr(Self::js_hex_decode),
        )
        .unwrap();

        // Crypto
        ctx.register_global_builtin_callable(
            js_string!("md5"),
            1,
            NativeFunction::from_fn_ptr(Self::js_md5),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("sha256"),
            1,
            NativeFunction::from_fn_ptr(Self::js_sha256),
        )
        .unwrap();

        // Util
        ctx.register_global_builtin_callable(
            js_string!("log"),
            1,
            NativeFunction::from_fn_ptr(Self::js_log),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("setHtml"),
            1,
            NativeFunction::from_fn_ptr(Self::js_set_html),
        )
        .unwrap();
    }

    // ========== Query ==========

    fn js_select_one(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let (parent, selector) = Self::parse_query_args(args, ctx)?;

        CURRENT_HTML.with(|cell| {
            let html_ref = cell.borrow();
            let html = html_ref
                .as_ref()
                .ok_or_else(|| JsNativeError::error().with_message("No document"))?;

            let sel = Selector::parse(&selector).map_err(|e| {
                JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
            })?;

            let element = if let Some((p_sel, p_idx)) = &parent {
                let p_selector = Selector::parse(p_sel).map_err(|e| {
                    JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
                })?;
                html.select(&p_selector)
                    .nth(*p_idx)
                    .and_then(|p| p.select(&sel).next())
            } else {
                html.select(&sel).next()
            };

            if let Some(el) = element {
                Ok(Self::make_element(
                    ctx,
                    &selector,
                    0,
                    parent,
                    el.value().name(),
                ))
            } else {
                Ok(JsValue::null())
            }
        })
    }

    fn js_select_all(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let (parent, selector) = Self::parse_query_args(args, ctx)?;

        CURRENT_HTML.with(|cell| {
            let html_ref = cell.borrow();
            let html = html_ref
                .as_ref()
                .ok_or_else(|| JsNativeError::error().with_message("No document"))?;

            let sel = Selector::parse(&selector).map_err(|e| {
                JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
            })?;

            let elements: Vec<JsValue> = if let Some((p_sel, p_idx)) = &parent {
                let p_selector = Selector::parse(p_sel).map_err(|e| {
                    JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
                })?;
                html.select(&p_selector)
                    .nth(*p_idx)
                    .map(|p| {
                        p.select(&sel)
                            .enumerate()
                            .map(|(i, el)| {
                                Self::make_element(
                                    ctx,
                                    &selector,
                                    i,
                                    parent.clone(),
                                    el.value().name(),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                html.select(&sel)
                    .enumerate()
                    .map(|(i, el)| Self::make_element(ctx, &selector, i, None, el.value().name()))
                    .collect()
            };

            Ok(JsValue::from(
                boa_engine::object::builtins::JsArray::from_iter(elements, ctx),
            ))
        })
    }

    fn parse_query_args(
        args: &[JsValue],
        ctx: &mut Context,
    ) -> JsResult<(Option<(String, usize)>, String)> {
        if args.len() >= 2 && args[0].is_object() {
            let parent = args[0].as_object().unwrap();
            let p_sel = parent
                .get(js_string!("_sel"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let p_idx = parent.get(js_string!("_idx"), ctx)?.to_i32(ctx)? as usize;
            let selector = args[1].to_string(ctx)?.to_std_string_escaped();
            Ok((Some((p_sel, p_idx)), selector))
        } else {
            Ok((
                None,
                args.get_or_undefined(0)
                    .to_string(ctx)?
                    .to_std_string_escaped(),
            ))
        }
    }

    fn make_element(
        ctx: &mut Context,
        selector: &str,
        index: usize,
        parent: Option<(String, usize)>,
        tag: &str,
    ) -> JsValue {
        let mut builder = ObjectInitializer::new(ctx);
        builder.property(
            js_string!("_sel"),
            js_string!(selector.to_string()),
            Attribute::all(),
        );
        builder.property(
            js_string!("_idx"),
            JsValue::from(index as i32),
            Attribute::all(),
        );
        builder.property(
            js_string!("tagName"),
            js_string!(tag.to_string()),
            Attribute::all(),
        );
        if let Some((p_sel, p_idx)) = parent {
            builder.property(js_string!("_psel"), js_string!(p_sel), Attribute::all());
            builder.property(
                js_string!("_pidx"),
                JsValue::from(p_idx as i32),
                Attribute::all(),
            );
        }
        JsValue::from(builder.build())
    }

    // ========== Element ==========

    fn get_element<F, R>(args: &[JsValue], ctx: &mut Context, f: F) -> JsResult<JsValue>
    where
        F: FnOnce(scraper::ElementRef) -> R,
        R: Into<JsValue>,
    {
        let obj = args
            .get_or_undefined(0)
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("Expected element"))?;
        let sel = obj
            .get(js_string!("_sel"), ctx)?
            .to_string(ctx)?
            .to_std_string_escaped();
        let idx = obj.get(js_string!("_idx"), ctx)?.to_i32(ctx)? as usize;
        let p_sel = obj.get(js_string!("_psel"), ctx)?;
        let p_idx = obj.get(js_string!("_pidx"), ctx)?;

        CURRENT_HTML.with(|cell| {
            let html_ref = cell.borrow();
            let html = html_ref
                .as_ref()
                .ok_or_else(|| JsNativeError::error().with_message("No document"))?;
            let selector = Selector::parse(&sel).map_err(|e| {
                JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
            })?;

            let el = if !p_sel.is_undefined() && !p_idx.is_undefined() {
                let ps = p_sel.to_string(ctx)?.to_std_string_escaped();
                let pi = p_idx.to_i32(ctx)? as usize;
                let p_selector = Selector::parse(&ps).map_err(|e| {
                    JsNativeError::syntax().with_message(format!("Invalid selector: {}", e))
                })?;
                html.select(&p_selector)
                    .nth(pi)
                    .and_then(|p| p.select(&selector).nth(idx))
            } else {
                html.select(&selector).nth(idx)
            };

            el.map(|e| f(e).into()).ok_or_else(|| {
                JsNativeError::error()
                    .with_message("Element not found")
                    .into()
            })
        })
    }

    fn js_text(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        Self::get_element(args, ctx, |el| {
            JsValue::from(js_string!(el.text().collect::<String>()))
        })
    }

    fn js_own_text(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        Self::get_element(args, ctx, |el| {
            let t: String = el
                .children()
                .filter_map(|c| {
                    if let Node::Text(t) = c.value() {
                        Some(t.text.as_ref())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            JsValue::from(js_string!(t))
        })
    }

    fn js_attr(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let attr_name = args
            .get_or_undefined(1)
            .to_string(ctx)?
            .to_std_string_escaped();
        Self::get_element(args, ctx, move |el| {
            el.value()
                .attr(&attr_name)
                .map(|v| JsValue::from(js_string!(v.to_string())))
                .unwrap_or(JsValue::null())
        })
    }

    fn js_html(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        Self::get_element(args, ctx, |el| JsValue::from(js_string!(el.inner_html())))
    }

    fn js_has_class(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let class = args
            .get_or_undefined(1)
            .to_string(ctx)?
            .to_std_string_escaped();
        Self::get_element(args, ctx, move |el| {
            JsValue::from(el.value().classes().any(|c| c == class))
        })
    }

    // ========== Encoding ==========

    fn js_base64_encode(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        Ok(JsValue::from(js_string!(BASE64.encode(s.as_bytes()))))
    }

    fn js_base64_decode(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        let decoded = BASE64
            .decode(s.as_bytes())
            .map_err(|e| JsNativeError::error().with_message(e.to_string()))?;
        let text = String::from_utf8(decoded)
            .map_err(|e| JsNativeError::error().with_message(e.to_string()))?;
        Ok(JsValue::from(js_string!(text)))
    }

    fn js_hex_encode(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        Ok(JsValue::from(js_string!(hex::encode(s.as_bytes()))))
    }

    fn js_hex_decode(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        let decoded =
            hex::decode(&s).map_err(|e| JsNativeError::error().with_message(e.to_string()))?;
        let text = String::from_utf8(decoded)
            .map_err(|e| JsNativeError::error().with_message(e.to_string()))?;
        Ok(JsValue::from(js_string!(text)))
    }

    // ========== Crypto ==========

    fn js_md5(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        Ok(JsValue::from(js_string!(hex::encode(Md5::digest(
            s.as_bytes()
        )))))
    }

    fn js_sha256(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        Ok(JsValue::from(js_string!(hex::encode(Sha256::digest(
            s.as_bytes()
        )))))
    }

    // ========== Util ==========

    fn js_log(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let out: Vec<String> = args
            .iter()
            .map(|v| {
                v.to_string(ctx)
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default()
            })
            .collect();
        println!("[JS] {}", out.join(" "));
        Ok(JsValue::undefined())
    }

    fn js_set_html(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        CURRENT_HTML.with(|cell| *cell.borrow_mut() = Some(Html::parse_document(&s)));
        Ok(JsValue::undefined())
    }
}

impl Default for JsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select() {
        let mut rt = JsRuntime::new();
        assert!(rt
            .execute("<h1>Hello</h1>", r#"text($("h1"))"#, None)
            .unwrap()
            .contains("Hello"));
    }

    #[test]
    fn test_select_all() {
        let mut rt = JsRuntime::new();
        assert_eq!(
            rt.execute("<ul><li>A</li><li>B</li></ul>", r#"$$("li").length"#, None)
                .unwrap(),
            "2"
        );
    }

    #[test]
    fn test_nested() {
        let mut rt = JsRuntime::new();
        let html = r#"<div id="a"><span>X</span></div><div id="b"><span>Y</span></div>"#;
        assert!(rt
            .execute(html, r##"text($($("#b"), "span"))"##, None)
            .unwrap()
            .contains("Y"));
    }

    #[test]
    fn test_vars() {
        let mut rt = JsRuntime::new();
        let mut vars = HashMap::new();
        vars.insert("foo".into(), "bar".into());
        assert!(rt.execute("", "foo", Some(&vars)).unwrap().contains("bar"));
    }

    #[test]
    fn test_base64() {
        let mut rt = JsRuntime::new();
        assert!(rt
            .execute("", r#"base64Decode(base64Encode("test"))"#, None)
            .unwrap()
            .contains("test"));
    }

    #[test]
    fn test_md5() {
        let mut rt = JsRuntime::new();
        assert!(rt
            .execute("", r#"md5("hello")"#, None)
            .unwrap()
            .contains("5d41402abc4b2a76b9719d911017c592"));
    }

    #[test]
    fn test_set_html() {
        let mut rt = JsRuntime::new();
        let result = rt
            .execute("", r#"setHtml("<p>New</p>"); text($("p"))"#, None)
            .unwrap();
        assert!(result.contains("New"));
    }
}
