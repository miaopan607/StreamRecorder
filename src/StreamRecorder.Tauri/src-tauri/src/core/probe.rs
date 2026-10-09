use parking_lot::Mutex;
use reqwest::{
    cookie::CookieStore,
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use url::Url;

pub const PC_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36 Edg/121.0.0.0";
pub const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 11; SAMSUNG SM-G973U) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/14.2 Chrome/87.0.4280.141 Mobile Safari/537.36";
#[derive(Clone, Default)]
pub struct ProbeInput {
    pub platform_key: String,
    pub live_url: String,
    pub quality: String,
    pub proxy: Option<String>,
    pub cookies: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub account_type: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamData {
    pub platform: Option<String>,
    pub anchor_name: Option<String>,
    pub is_live: bool,
    pub title: Option<String>,
    pub quality: Option<String>,
    pub m3u8_url: Option<String>,
    pub flv_url: Option<String>,
    pub record_url: Option<String>,
    pub new_cookies: Option<String>,
    pub new_token: Option<String>,
    pub extra: Option<Value>,
    pub live_url: Option<String>,
}
impl StreamData {
    pub fn new(input: &ProbeInput, platform: &str, anchor: String, is_live: bool) -> Self {
        Self {
            platform: Some(platform.into()),
            anchor_name: Some(anchor),
            is_live,
            live_url: Some(input.live_url.clone()),
            quality: Some(input.quality.clone()),
            ..Self::default()
        }
    }
    pub fn urls(&mut self, hls: String, flv: String, prefer_flv: bool) {
        self.record_url = if prefer_flv && !flv.is_empty() {
            Some(flv.clone())
        } else if !hls.is_empty() {
            Some(hls.clone())
        } else if !flv.is_empty() {
            Some(flv.clone())
        } else {
            None
        };
        self.m3u8_url = (!hls.is_empty()).then_some(hls);
        self.flv_url = (!flv.is_empty()).then_some(flv);
    }
    pub fn validate(self) -> Result<Self, String> {
        if self.is_live
            && self.record_url.as_deref().unwrap_or("").is_empty()
            && self.m3u8_url.as_deref().unwrap_or("").is_empty()
            && self.flv_url.as_deref().unwrap_or("").is_empty()
        {
            Err("没有可用的直播流地址".into())
        } else {
            Ok(self)
        }
    }
}
#[derive(Default)]
pub struct Session {
    pub fields: tokio::sync::Mutex<BTreeMap<String, String>>,
    pub jar: Arc<reqwest::cookie::Jar>,
}
#[derive(Hash, PartialEq, Eq)]
struct SessionKey {
    platform: String,
    proxy: Option<String>,
    cookies: Option<String>,
    username: Option<String>,
    password: Option<String>,
    account_type: Option<String>,
}
#[derive(Default)]
pub struct ProbeService {
    clients: Mutex<HashMap<(Option<String>, bool), Client>>,
    sessions: Mutex<HashMap<SessionKey, Arc<Session>>>,
    #[cfg(test)]
    pub endpoint: Mutex<Option<String>>,
}
impl ProbeService {
    pub fn invalidate_sessions(&self) {
        self.sessions.lock().clear();
    }
    pub async fn probe(
        &self,
        input: ProbeInput,
        cancel: CancellationToken,
    ) -> Result<StreamData, String> {
        if input.platform_key == "custom" {
            let mut data = StreamData::new(&input, "自定义直播流", "直播间".into(), true);
            data.title = Some("自定义直播流".into());
            data.record_url = Some(input.live_url.clone());
            let url = input.live_url.to_ascii_lowercase();
            if url.contains(".m3u8") {
                data.m3u8_url = Some(input.live_url.clone());
            }
            if url.contains(".flv") {
                data.flv_url = Some(input.live_url.clone());
            }
            return data.validate();
        }
        let http1 = matches!(input.platform_key.as_str(), "tiktok" | "twitch" | "faceit");
        let pool_key = (input.proxy.clone(), http1);
        let client = {
            let mut clients = self.clients.lock();
            if let Some(client) = clients.get(&pool_key) {
                client.clone()
            } else {
                let mut builder = Client::builder()
                    .connect_timeout(Duration::from_secs(5))
                    .timeout(Duration::from_secs(20))
                    .redirect(reqwest::redirect::Policy::none());
                // Cookie Jar 属于凭据会话，不与连接池共同跨账号共享。
                builder = builder.no_proxy();
                if http1 {
                    builder = builder.http1_only();
                }
                if let Some(proxy) = input.proxy.as_deref() {
                    builder = builder
                        .proxy(reqwest::Proxy::all(proxy).map_err(|_| "代理地址无效".to_string())?);
                }
                let client = builder.build().map_err(|e| e.to_string())?;
                clients.insert(pool_key, client.clone());
                client
            }
        };
        let key = SessionKey {
            platform: input.platform_key.clone(),
            proxy: input.proxy.clone(),
            cookies: input.cookies.clone(),
            username: input.username.clone(),
            password: input.password.clone(),
            account_type: input.account_type.clone(),
        };
        let session = self.sessions.lock().entry(key).or_default().clone();
        let context = ProbeContext {
            client,
            cookies: input.cookies.clone(),
            cancel: cancel.clone(),
            session,
            #[cfg(test)]
            endpoint: self.endpoint.lock().clone(),
        };
        let operation = super::platforms::probe(&context, &input);
        tokio::select! {
            _ = cancel.cancelled() => Err("探测已取消".into()),
            result = tokio::time::timeout(Duration::from_secs(60), operation) => result.map_err(|_| "平台探测超时".to_string())?.and_then(StreamData::validate).map_err(|e| format!("{}：{e}", input.platform_key)),
        }
    }
}
pub struct ProbeContext {
    pub client: Client,
    pub cookies: Option<String>,
    pub cancel: CancellationToken,
    pub session: Arc<Session>,
    #[cfg(test)]
    pub endpoint: Option<String>,
}
pub enum Body {
    None,
    Form(Vec<(String, String)>),
    Json(Value),
    Bytes(Vec<u8>),
}
pub struct Response {
    pub text: String,
    pub final_url: String,
    pub cookies: BTreeMap<String, String>,
}
pub struct RawResponse {
    pub bytes: Vec<u8>,
    pub final_url: String,
    pub cookies: BTreeMap<String, String>,
}
impl ProbeContext {
    pub async fn request_raw(
        &self,
        method: Method,
        url: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> Result<RawResponse, String> {
        let mut logical = Url::parse(url).map_err(|_| "平台请求 URL 无效".to_string())?;
        let mut map = HeaderMap::new();
        for (name, value) in [
            ("user-agent", PC_UA),
            (
                "accept-language",
                "zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2",
            ),
        ] {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).map_err(|e| e.to_string())?,
            );
        }
        if let Some(cookie) = self.cookies.as_deref() {
            map.insert(
                "cookie",
                HeaderValue::from_str(cookie).map_err(|_| "Cookie 包含无效字符".to_string())?,
            );
        }
        if let Some(cookie) = self.session.jar.cookies(&logical) {
            let mut fields = BTreeMap::new();
            for part in self
                .cookies
                .as_deref()
                .unwrap_or("")
                .split(';')
                .chain(cookie.to_str().unwrap_or("").split(';'))
            {
                if let Some((key, value)) = part.trim().split_once('=') {
                    fields.insert(key.to_owned(), value.to_owned());
                }
            }
            map.insert(
                "cookie",
                HeaderValue::from_str(&cookie_string(&fields))
                    .map_err(|_| "Cookie 包含无效字符".to_string())?,
            );
        }
        for &(name, value) in headers {
            map.insert(
                HeaderName::from_bytes(name.as_bytes()).map_err(|e| e.to_string())?,
                HeaderValue::from_str(value).map_err(|_| "请求 header 包含无效字符".to_string())?,
            );
        }
        let operation = async {
            let mut body = Some(body);
            let mut cookies = BTreeMap::new();
            for redirect in 0..=10 {
                #[cfg(test)]
                let target = if let Some(endpoint) = &self.endpoint {
                    format!(
                        "{endpoint}{}{}",
                        logical.path(),
                        logical.query().map(|q| format!("?{q}")).unwrap_or_default()
                    )
                } else {
                    logical.to_string()
                };
                #[cfg(not(test))]
                let target = logical.as_str();
                let mut builder = self
                    .client
                    .request(method.clone(), target)
                    .headers(map.clone());
                builder = match body.take().unwrap_or(Body::None) {
                    Body::None => builder,
                    Body::Form(fields) => builder.form(&fields),
                    Body::Json(value) => builder.json(&value),
                    Body::Bytes(value) => builder.body(value),
                };
                let mut response = builder
                    .send()
                    .await
                    .map_err(|e| format!("网络请求失败：{}", e.without_url()))?;
                let status = response.status().as_u16();
                for cookie in response.headers().get_all("set-cookie") {
                    if let Ok(value) = cookie.to_str() {
                        self.session.jar.add_cookie_str(value, &logical);
                    }
                }
                cookies.extend(
                    response
                        .cookies()
                        .map(|c| (c.name().to_owned(), c.value().to_owned())),
                );
                // 上游 GET 跟随重定向；POST 保留首次响应的登录 Cookie。
                if response.status().is_redirection()
                    && matches!(method, Method::GET | Method::HEAD)
                {
                    if redirect == 10 {
                        return Err("平台重定向次数过多".into());
                    }
                    let location = response
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .ok_or("平台重定向地址缺失")?;
                    let next = logical
                        .join(location)
                        .map_err(|_| "平台重定向地址无效".to_string())?;
                    if !matches!(next.scheme(), "http" | "https") {
                        return Err("平台重定向协议无效".into());
                    }
                    if next.host_str() != logical.host_str() {
                        map.remove("cookie");
                        map.remove("authorization");
                    }
                    if let Some(cookie) = self.session.jar.cookies(&next) {
                        map.insert("cookie", cookie);
                    }
                    logical = next;
                    continue;
                }
                if !response.status().is_success() && !response.status().is_redirection() {
                    return Err(format!("平台 HTTP 状态 {status}"));
                }
                let maximum = 16 * 1024 * 1024;
                if response
                    .content_length()
                    .is_some_and(|n| n > maximum as u64)
                {
                    return Err("平台响应过大".into());
                }
                let mut bytes = Vec::with_capacity(
                    response.content_length().unwrap_or(0).min(maximum as u64) as usize,
                );
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|e| format!("平台响应读取失败：{}", e.without_url()))?
                {
                    if bytes.len() + chunk.len() > maximum {
                        return Err("平台响应过大".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                return Ok(RawResponse {
                    bytes,
                    final_url: logical.to_string(),
                    cookies,
                });
            }
            Err("平台重定向次数过多".into())
        };
        tokio::select! { _ = self.cancel.cancelled() => Err("探测已取消".into()), result = operation => result }
    }
    pub async fn request(
        &self,
        method: Method,
        url: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> Result<Response, String> {
        let response = self.request_raw(method, url, headers, body).await?;
        let text =
            String::from_utf8(response.bytes).map_err(|_| "平台响应不是有效 UTF-8".to_string())?;
        Ok(Response {
            text,
            final_url: response.final_url,
            cookies: response.cookies,
        })
    }
    pub async fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
        Ok(self
            .request(Method::GET, url, headers, Body::None)
            .await?
            .text)
    }
    pub async fn json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> Result<Value, String> {
        let response = self
            .request(
                if matches!(body, Body::None) {
                    Method::GET
                } else {
                    Method::POST
                },
                url,
                headers,
                body,
            )
            .await?;
        parse_json(&response.text)
    }
    pub async fn variants(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<Vec<String>, String> {
        let text = self.get(url, headers).await?;
        if !text.trim_start().starts_with("#EXTM3U") {
            return Err("平台未返回有效 HLS 清单".into());
        }
        let base = Url::parse(url).map_err(|e| e.to_string())?;
        let mut results = vec![];
        let mut bandwidth = None;
        for line in text.lines().map(str::trim) {
            if line.starts_with("#EXT-X-STREAM-INF:") {
                bandwidth = Some(
                    capture(r"BANDWIDTH=(\d+)", line)?
                        .parse::<u64>()
                        .map_err(|_| "HLS 带宽无效".to_string())?,
                );
            } else if let Some(rate) = bandwidth {
                if !line.is_empty() && !line.starts_with('#') {
                    results.push((
                        rate,
                        base.join(line).map_err(|e| e.to_string())?.to_string(),
                    ));
                    bandwidth = None;
                }
            }
        }
        results.sort_by(|a, b| b.0.cmp(&a.0));
        if results.is_empty() {
            Ok(vec![url.to_owned()])
        } else {
            Ok(results.into_iter().map(|(_, url)| url).collect())
        }
    }
}
pub fn at<'a>(value: &'a Value, path: &str) -> Result<&'a Value, String> {
    value
        .pointer(path)
        .filter(|v| !v.is_null())
        .ok_or_else(|| format!("平台响应缺少字段 {path}"))
}
pub fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
pub fn field(value: &Value, path: &str) -> Result<String, String> {
    Ok(text(at(value, path)?))
}
pub fn optional(value: &Value, path: &str) -> String {
    value.pointer(path).map(text).unwrap_or_default()
}
pub fn truth(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}
pub fn array(value: &Value) -> Result<&Vec<Value>, String> {
    value.as_array().ok_or("平台响应不是列表".into())
}
pub fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, String> {
    value.as_object().ok_or("平台响应不是对象".into())
}
pub fn quality_index(quality: &str) -> usize {
    match quality.to_ascii_uppercase().as_str() {
        "UHD" | "1" => 1,
        "HD" | "2" => 2,
        "SD" | "3" => 3,
        "LD" | "4" => 4,
        _ => 0,
    }
}
pub fn pick<'a, T>(values: &'a [T], quality: &str) -> Result<&'a T, String> {
    if values.is_empty() {
        Err("平台没有可用画质".into())
    } else {
        Ok(&values[quality_index(quality).min(values.len() - 1)])
    }
}
pub fn room(url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|_| "直播 URL 无效".to_string())?;
    parsed
        .path_segments()
        .and_then(|s| s.filter(|s| !s.is_empty()).next_back())
        .map(str::to_owned)
        .ok_or("直播 URL 缺少房间 ID".into())
}
pub fn query(url: &str, key: &str) -> Result<String, String> {
    Url::parse(url)
        .map_err(|_| "直播 URL 无效".to_string())?
        .query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| format!("直播 URL 缺少 {key}"))
}
pub fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
pub fn unquote(value: &str) -> String {
    url::form_urlencoded::parse(format!("v={}", value.replace('+', "%2B")).as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}
pub fn form(fields: &[(&str, &str)]) -> Body {
    Body::Form(
        fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}
static EXPRESSIONS: LazyLock<Mutex<HashMap<&'static str, Arc<regex::Regex>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn expression(pattern: &'static str) -> Result<Arc<regex::Regex>, String> {
    let mut expressions = EXPRESSIONS.lock();
    if let Some(value) = expressions.get(pattern) {
        return Ok(value.clone());
    }
    let value = Arc::new(regex::Regex::new(pattern).map_err(|e| e.to_string())?);
    expressions.insert(pattern, value.clone());
    Ok(value)
}
pub fn capture(pattern: &'static str, text: &str) -> Result<String, String> {
    expression(pattern)?
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|c| c.as_str().to_owned())
        .ok_or("平台页面缺少必要数据".into())
}
pub fn captures(pattern: &'static str, text: &str) -> Result<Vec<String>, String> {
    Ok(expression(pattern)?
        .captures_iter(text)
        .filter_map(|c| c.get(1).map(|c| c.as_str().to_owned()))
        .collect())
}
pub fn parse_json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|_| "平台 JSON 解析失败".into())
}
pub fn cookie_string(cookies: &BTreeMap<String, String>) -> String {
    cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}
