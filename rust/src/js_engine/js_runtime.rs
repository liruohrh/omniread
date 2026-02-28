//! JavaScript runtime for HTML parsing using boa_engine

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use boa_engine::{
    js_string, object::ObjectInitializer, property::Attribute, Context, JsArgs, JsNativeError,
    JsResult, JsValue, NativeFunction, Source,
};
use md5::{Digest as Md5Digest, Md5};
use scraper::{Html, Node, Selector};
use sha2::Sha256;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

use super::context::ParseContext;
use crate::http_client::HttpClient;
use crate::request_config::{RequestConfig, RequestMethod};

thread_local! {
    static CURRENT_HTML: RefCell<Option<Html>> = RefCell::new(None);
    /// Shared variables storage for JS access
    static SHARED_VARS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// WebView request storage
    static WEBVIEW_REQUEST: RefCell<Option<WebViewRequest>> = RefCell::new(None);
    /// Next page URL for multi-page content
    static NEXT_PAGE_URL: RefCell<Option<String>> = RefCell::new(None);
    /// HTTP client for JS API
    static HTTP_CLIENT: RefCell<Option<Arc<HttpClient>>> = RefCell::new(None);
}

/// WebView request from JS
#[derive(Debug, Clone)]
pub struct WebViewRequest {
    pub url: String,
    pub js: Option<String>,
    pub wait_for: Option<String>,
}

/// HTTP response for JS
#[derive(Debug, Clone)]
pub struct JsHttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(_) => {
            let runtime = Runtime::new().expect("Failed to create Tokio runtime");
            runtime.block_on(future)
        }
        Err(_) => {
            let runtime = Runtime::new().expect("Failed to create Tokio runtime");
            runtime.block_on(future)
        }
    }
}

fn do_get(client: &HttpClient, url: &str) -> Result<JsHttpResponse, String> {
    let config = RequestConfig::new(url.to_string());
    let resp = block_on(client.execute_full(config))
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    Ok(JsHttpResponse {
        status: resp.status,
        body: resp.body,
        headers: resp.headers,
    })
}

fn do_post(
    client: &HttpClient,
    url: &str,
    body: Option<&str>,
    content_type: Option<&str>,
) -> Result<JsHttpResponse, String> {
    let mut config = RequestConfig::new(url.to_string());
    config.method = RequestMethod::Post;
    config.content_type = content_type.map(|s| s.to_string());
    config.body = body.and_then(|b| serde_json::from_str(b).ok());
    
    let resp = block_on(client.execute_full(config))
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    Ok(JsHttpResponse {
        status: resp.status,
        body: resp.body,
        headers: resp.headers,
    })
}

fn do_request(
    client: &HttpClient,
    method: &str,
    url: &str,
    headers: HashMap<String, String>,
    body: Option<&str>,
    content_type: Option<&str>,
) -> Result<JsHttpResponse, String> {
    let mut config = RequestConfig::new(url.to_string());
    config.method = match method.to_uppercase().as_str() {
        "GET" => RequestMethod::Get,
        "POST" => RequestMethod::Post,
        "PUT" => RequestMethod::Put,
        "DELETE" => RequestMethod::Delete,
        "PATCH" => RequestMethod::Patch,
        _ => RequestMethod::Get,
    };
    config.headers = headers;
    config.content_type = content_type.map(|s| s.to_string());
    config.body = body.and_then(|b| serde_json::from_str(b).ok());
    
    let resp = block_on(client.execute_full(config))
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    Ok(JsHttpResponse {
        status: resp.status,
        body: resp.body,
        headers: resp.headers,
    })
}

/// JS runtime with DOM query APIs for HTML parsing.
pub struct JsRuntime {
    context: Context,
}

/// Extended execution result
#[derive(Debug)]
pub struct ExecuteResult {
    pub json: String,
    pub shared_updates: HashMap<String, String>,
    pub webview_request: Option<WebViewRequest>,
    pub next_page_url: Option<String>,
}

impl JsRuntime {
    pub fn new() -> Self {
        let mut context = Context::default();
        Self::register_api(&mut context);
        HTTP_CLIENT.with(|cell| *cell.borrow_mut() = None);
        Self { context }
    }

    pub fn with_http_client(http_client: HttpClient) -> Self {
        let rt = Self::new();
        HTTP_CLIENT.with(|cell| *cell.borrow_mut() = Some(Arc::new(http_client)));
        rt
    }

    pub fn set_http_client(&mut self, http_client: HttpClient) {
        HTTP_CLIENT.with(|cell| *cell.borrow_mut() = Some(Arc::new(http_client)));
    }

    /// Execute JS script with parse context
    pub fn execute_with_context(
        &mut self,
        html: &str,
        script: &str,
        ctx: &ParseContext,
    ) -> Result<ExecuteResult, String> {
        SHARED_VARS.with(|cell| {
            *cell.borrow_mut() = ctx.shared.clone();
        });
        WEBVIEW_REQUEST.with(|cell| *cell.borrow_mut() = None);
        NEXT_PAGE_URL.with(|cell| *cell.borrow_mut() = None);

        let init_script = Self::build_init_script(ctx);
        let full_script = format!("{}\n{}", init_script, script);
        let result = self.execute(html, &full_script, Some(&ctx.to_vars()))?;

        let shared_updates = SHARED_VARS.with(|cell| cell.borrow().clone());
        let webview_request = WEBVIEW_REQUEST.with(|cell| cell.borrow().clone());
        let next_page_url = NEXT_PAGE_URL.with(|cell| cell.borrow().clone());

        Ok(ExecuteResult {
            json: result,
            shared_updates,
            webview_request,
            next_page_url,
        })
    }

    fn build_init_script(_ctx: &ParseContext) -> String {
        let mut lines = Vec::new();
        lines.push(
            "var book = typeof __book_json !== 'undefined' ? JSON.parse(__book_json) : null;"
                .to_string(),
        );
        lines.push(
            "var source = typeof __source_json !== 'undefined' ? JSON.parse(__source_json) : null;"
                .to_string(),
        );
        lines.push(
            "var page = typeof __page_json !== 'undefined' ? JSON.parse(__page_json) : null;"
                .to_string(),
        );
        lines.push(
            "var parseMode = typeof __parse_mode !== 'undefined' ? __parse_mode : 'book_detail';"
                .to_string(),
        );
        lines.push("var shared = { _data: typeof __shared_json !== 'undefined' ? JSON.parse(__shared_json) : {} };".to_string());
        lines.join("\n")
    }

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

        ctx.register_global_builtin_callable(
            js_string!("sharedGet"),
            1,
            NativeFunction::from_fn_ptr(Self::js_shared_get),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("sharedSet"),
            2,
            NativeFunction::from_fn_ptr(Self::js_shared_set),
        )
        .unwrap();

        ctx.register_global_builtin_callable(
            js_string!("webview"),
            1,
            NativeFunction::from_fn_ptr(Self::js_webview),
        )
        .unwrap();

        ctx.register_global_builtin_callable(
            js_string!("setNextPage"),
            1,
            NativeFunction::from_fn_ptr(Self::js_set_next_page),
        )
        .unwrap();

        ctx.register_global_builtin_callable(
            js_string!("httpGet"),
            1,
            NativeFunction::from_fn_ptr(Self::js_http_get),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("httpPost"),
            2,
            NativeFunction::from_fn_ptr(Self::js_http_post),
        )
        .unwrap();
        ctx.register_global_builtin_callable(
            js_string!("httpRequest"),
            1,
            NativeFunction::from_fn_ptr(Self::js_http_request),
        )
        .unwrap();
    }

    fn make_response_obj(ctx: &mut Context, resp: &JsHttpResponse) -> JsValue {
        let headers_obj = {
            let mut builder = ObjectInitializer::new(ctx);
            for (key, value) in &resp.headers {
                builder.property(js_string!(key.clone()), js_string!(value.clone()), Attribute::all());
            }
            builder.build()
        };
        
        let mut obj = ObjectInitializer::new(ctx);
        obj.property(js_string!("status"), JsValue::from(resp.status as i32), Attribute::all());
        obj.property(js_string!("body"), js_string!(resp.body.clone()), Attribute::all());
        obj.property(js_string!("headers"), headers_obj, Attribute::all());
        
        JsValue::from(obj.build())
    }

    fn get_http_client() -> Result<Arc<HttpClient>, JsNativeError> {
        HTTP_CLIENT.with(|cell| {
            cell.borrow()
                .clone()
                .ok_or_else(|| JsNativeError::error().with_message("HTTP client not set. Use JsRuntime::with_http_client() or set_http_client()"))
        })
    }

    fn js_http_get(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let url = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
        let client = Self::get_http_client()?;
        let resp = do_get(&client, &url).map_err(|e| JsNativeError::error().with_message(e))?;
        Ok(Self::make_response_obj(ctx, &resp))
    }

    fn js_http_post(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let url = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
        let body_arg = args.get_or_undefined(1);
        
        let (body, content_type) = if body_arg.is_undefined() || body_arg.is_null() {
            (None, None)
        } else if body_arg.is_object() {
            let json = body_arg.to_json(ctx)?;
            (Some(json.to_string()), Some("application/json".to_string()))
        } else {
            (Some(body_arg.to_string(ctx)?.to_std_string_escaped()), Some("text/plain".to_string()))
        };

        let client = Self::get_http_client()?;
        let resp = do_post(&client, &url, body.as_deref(), content_type.as_deref())
            .map_err(|e| JsNativeError::error().with_message(e))?;
        Ok(Self::make_response_obj(ctx, &resp))
    }

    fn js_http_request(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let arg = args.get_or_undefined(0);
        if let Some(obj) = arg.as_object() {
            let url = obj.get(js_string!("url"), ctx)?.to_string(ctx)?.to_std_string_escaped();
            let method_val = obj.get(js_string!("method"), ctx)?;
            let headers_val = obj.get(js_string!("headers"), ctx)?;
            let content_type_val = obj.get(js_string!("contentType"), ctx)?;
            let body_val = obj.get(js_string!("body"), ctx)?;

            let method = if method_val.is_undefined() || method_val.is_null() {
                "GET".to_string()
            } else {
                method_val.to_string(ctx)?.to_std_string_escaped()
            };

            let mut headers = HashMap::new();
            if !headers_val.is_undefined() && !headers_val.is_null() {
                if let Ok(headers_json) = headers_val.to_json(ctx) {
                    if let Some(hobj) = headers_json.as_object() {
                        for (key, value) in hobj {
                            if let Some(s) = value.as_str() {
                                headers.insert(key.to_string(), s.to_string());
                            }
                        }
                    }
                }
            }

            let content_type = if content_type_val.is_undefined() || content_type_val.is_null() {
                None
            } else {
                Some(content_type_val.to_string(ctx)?.to_std_string_escaped())
            };

            let body = if body_val.is_undefined() || body_val.is_null() {
                None
            } else if body_val.is_object() {
                Some(body_val.to_json(ctx)?.to_string())
            } else {
                Some(body_val.to_string(ctx)?.to_std_string_escaped())
            };

            let client = Self::get_http_client()?;
            let resp = do_request(&client, &method, &url, headers, body.as_deref(), content_type.as_deref())
                .map_err(|e| JsNativeError::error().with_message(e))?;
            Ok(Self::make_response_obj(ctx, &resp))
        } else {
            Err(JsNativeError::typ().with_message("httpRequest requires options object").into())
        }
    }

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

    fn js_shared_get(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let key = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        SHARED_VARS.with(|cell| {
            let vars = cell.borrow();
            match vars.get(&key) {
                Some(v) => Ok(JsValue::from(js_string!(v.clone()))),
                None => Ok(JsValue::null()),
            }
        })
    }

    fn js_shared_set(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let key = args
            .get_or_undefined(0)
            .to_string(ctx)?
            .to_std_string_escaped();
        let value = args
            .get_or_undefined(1)
            .to_string(ctx)?
            .to_std_string_escaped();
        SHARED_VARS.with(|cell| {
            cell.borrow_mut().insert(key, value);
        });
        Ok(JsValue::undefined())
    }

    fn js_webview(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let arg = args.get_or_undefined(0);

        if arg.is_string() {
            let url = arg.to_string(ctx)?.to_std_string_escaped();
            WEBVIEW_REQUEST.with(|cell| {
                *cell.borrow_mut() = Some(WebViewRequest {
                    url,
                    js: None,
                    wait_for: None,
                });
            });
        } else if let Some(obj) = arg.as_object() {
            let url = obj
                .get(js_string!("url"), ctx)?
                .to_string(ctx)?
                .to_std_string_escaped();
            let js_val = obj.get(js_string!("js"), ctx)?;
            let wait_for_val = obj.get(js_string!("waitFor"), ctx)?;

            let js_str = if js_val.is_undefined() || js_val.is_null() {
                None
            } else {
                Some(js_val.to_string(ctx)?.to_std_string_escaped())
            };

            let wait_for_str = if wait_for_val.is_undefined() || wait_for_val.is_null() {
                None
            } else {
                Some(wait_for_val.to_string(ctx)?.to_std_string_escaped())
            };

            WEBVIEW_REQUEST.with(|cell| {
                *cell.borrow_mut() = Some(WebViewRequest {
                    url,
                    js: js_str,
                    wait_for: wait_for_str,
                });
            });
        } else {
            return Err(JsNativeError::typ()
                .with_message("webview requires url string or options object")
                .into());
        }

        Ok(JsValue::null())
    }

    fn js_set_next_page(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let arg = args.get_or_undefined(0);
        if arg.is_null() || arg.is_undefined() {
            NEXT_PAGE_URL.with(|cell| *cell.borrow_mut() = None);
        } else {
            let url = arg.to_string(ctx)?.to_std_string_escaped();
            NEXT_PAGE_URL.with(|cell| *cell.borrow_mut() = Some(url));
        }
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
    use mockito::mock;

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

    #[test]
    fn test_http_get_with_mock_server() {
        let client = HttpClient::new().unwrap();
        
        let _m = mock("GET", "/test")
            .with_status(200)
            .with_header("Content-Type", "text/plain")
            .with_body("Hello, World!")
            .create();

        let mut rt = JsRuntime::with_http_client(client);
        let ctx = ParseContext::default();
        let script = format!(r#"
            let resp = httpGet("{}");
            resp.body;
        "#, format!("{}/test", mockito::server_url()));
        
        let result = rt.execute_with_context("", &script, &ctx).unwrap();
        assert_eq!(result.json, "\"Hello, World!\"");
    }

    #[test]
    fn test_http_post_with_mock_server() {
        let client = HttpClient::new().unwrap();
        
        let _m = mock("POST", "/api")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"status": "ok"}"#)
            .create();

        let mut rt = JsRuntime::with_http_client(client);
        let ctx = ParseContext::default();
        let script = format!(r#"
            let resp = httpPost("{}", {{ key: "value" }});
            resp.body;
        "#, format!("{}/api", mockito::server_url()));
        
        let result = rt.execute_with_context("", &script, &ctx).unwrap();
        assert!(result.json.contains("ok"));
    }

    #[test]
    fn test_http_request_full_with_mock_server() {
        let client = HttpClient::new().unwrap();
        
        let _m = mock("PUT", "/api/v2")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"data": "test"}"#)
            .create();

        let mut rt = JsRuntime::with_http_client(client);
        let ctx = ParseContext::default();
        let script = format!(r#"
            let resp = httpRequest({{
                url: "{}",
                method: "PUT",
                headers: {{ "X-Custom": "test" }},
                contentType: "application/json",
                body: {{ data: "test" }}
            }});
            JSON.stringify({{ status: resp.status, body: resp.body }});
        "#, format!("{}/api/v2", mockito::server_url()));
        
        let result = rt.execute_with_context("", &script, &ctx).unwrap();
        assert!(result.json.contains("200"));
        assert!(result.json.contains("test"));
    }
}
