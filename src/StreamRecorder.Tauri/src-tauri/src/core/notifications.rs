use super::models::{CoreSettings, RecordingJob};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use parking_lot::Mutex;
use reqwest::Client;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy)]
pub enum NotificationEvent {
    Start,
    End,
    Error,
}
#[derive(Default)]
pub struct NotificationService {
    clients: Mutex<HashMap<Option<String>, Client>>,
}
fn entries(text: &str) -> impl Iterator<Item = &str> {
    text.split([',', '，'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
pub fn proxy(settings: &CoreSettings) -> Option<String> {
    if !settings.enable_proxy || settings.proxy_address.trim().is_empty() {
        return None;
    }
    let address = settings.proxy_address.trim();
    Some(if address.starts_with("http") {
        address.to_owned()
    } else {
        format!("http://{address}")
    })
}
pub fn message(
    settings: &CoreSettings,
    job: &RecordingJob,
    event: NotificationEvent,
) -> (String, String) {
    let title = if settings.custom_notification_title.trim().is_empty() {
        format!("StreamRecorder - {}", job.input.streamer_name)
    } else {
        settings.custom_notification_title.trim().to_owned()
    };
    let (template, default) = match event {
        NotificationEvent::Start => (
            &settings.custom_stream_start_content,
            if settings.only_notify_no_record || job.input.only_notify_no_record {
                format!("检测到 {} 正在直播。", job.input.streamer_name)
            } else {
                format!("检测到 {} 正在直播，已开始录制。", job.input.streamer_name)
            },
        ),
        NotificationEvent::End => (
            &settings.custom_stream_end_content,
            format!("{} 的录制已结束。", job.input.streamer_name),
        ),
        NotificationEvent::Error => (
            &settings.custom_stream_error_content,
            format!(
                "{} 的录制异常结束：{}",
                job.input.streamer_name,
                if job.error_message.is_empty() {
                    "请查看日志。"
                } else {
                    &job.error_message
                }
            ),
        ),
    };
    let content = if template.trim().is_empty() {
        default
    } else {
        template.trim().to_owned()
    };
    (
        title,
        content
            .replace("{streamer_name}", &job.input.streamer_name)
            .replace(
                "{title}",
                if job.live_title.is_empty() {
                    &job.title
                } else {
                    &job.live_title
                },
            )
            .replace("{error}", &job.error_message),
    )
}
impl NotificationService {
    fn client(&self, proxy: Option<String>) -> Result<Client, String> {
        let mut clients = self.clients.lock();
        if let Some(client) = clients.get(&proxy) {
            return Ok(client.clone());
        }
        let mut builder = Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none());
        if let Some(address) = proxy.as_deref() {
            builder = builder
                .proxy(reqwest::Proxy::all(address).map_err(|_| "通知代理地址无效".to_string())?);
        }
        let client = builder
            .build()
            .map_err(|_| "无法初始化通知网络服务".to_string())?;
        clients.insert(proxy, client.clone());
        Ok(client)
    }
    async fn post(
        &self,
        url: &str,
        payload: &Value,
        proxy: Option<String>,
    ) -> Result<Value, String> {
        let mut response = self
            .client(proxy)?
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|error| format!("网络发送失败：{}", error.without_url()))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status().as_u16()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "通知响应读取失败".to_string())?
        {
            if bytes.len() + chunk.len() > 16 * 1024 * 1024 {
                return Err("通知响应过大".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| "通知接口未返回有效 JSON 确认".into())
    }
    fn acknowledge(channel: &str, response: &Value) -> Result<(), String> {
        let explicit = response.get("success").and_then(Value::as_bool) == Some(true);
        let success = match channel {
            "钉钉" => response.get("errcode").and_then(Value::as_i64) == Some(0) || explicit,
            "微信" | "Bark" => {
                response.get("code").and_then(Value::as_i64) == Some(200) || explicit
            }
            "Telegram" => response.get("ok").and_then(Value::as_bool) == Some(true),
            "ntfy" => {
                response.get("error").is_none()
                    && response
                        .get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| !id.is_empty())
                    && response.get("event").and_then(Value::as_str) == Some("message")
            }
            "Server酱" => response.get("code").and_then(Value::as_i64) == Some(0) || explicit,
            _ => false,
        };
        if success {
            Ok(())
        } else {
            let code = response
                .get("errcode")
                .or_else(|| response.get("code"))
                .and_then(Value::as_i64);
            Err(code.map_or_else(
                || "接口拒绝了通知".into(),
                |code| format!("接口拒绝了通知，错误码 {code}"),
            ))
        }
    }
    async fn channel(
        &self,
        channel: &str,
        settings: &CoreSettings,
        title: &str,
        content: &str,
    ) -> Result<(), String> {
        if channel == "邮件" {
            return email(settings, title, content).await;
        }
        let mut requests = Vec::new();
        match channel {
            "钉钉" => {
                for url in entries(&settings.dingtalk_webhook_url) {
                    requests.push((url.to_owned(),json!({"msgtype":"text","text":{"content":content},"at":{"atMobiles":if settings.dingtalk_at_objects.is_empty(){Vec::<&str>::new()}else{vec![settings.dingtalk_at_objects.as_str()]},"isAtAll":settings.dingtalk_at_all}})));
                }
            }
            "微信" => {
                for url in entries(&settings.wechat_webhook_url) {
                    requests.push((url.to_owned(), json!({"title":title,"content":content})));
                }
            }
            "Bark" => {
                for url in entries(&settings.bark_webhook_url) {
                    requests.push((url.to_owned(),json!({"title":title,"body":content,"level":if settings.bark_interrupt_level.is_empty(){"active"}else{&settings.bark_interrupt_level},"sound":settings.bark_sound})));
                }
            }
            "ntfy" => {
                let mut tags = entries(&settings.ntfy_tags).collect::<Vec<_>>();
                if tags.is_empty() {
                    tags.push("tada");
                }
                for url in entries(&settings.ntfy_server_url) {
                    requests.push((url.to_owned(),json!({"title":title,"message":content,"tags":tags,"actions":if settings.ntfy_action_url.is_empty(){json!([])}else{json!([{"action":"view","label":"open","url":settings.ntfy_action_url}])},"email":settings.ntfy_email})));
                }
            }
            "Telegram" => requests.push((
                format!(
                    "https://api.telegram.org/bot{}/sendMessage",
                    settings.telegram_api_token
                ),
                json!({"chat_id":settings.telegram_chat_id,"text":content}),
            )),
            "Server酱" => {
                for key in entries(&settings.serverchan_sendkey) {
                    let key_encoded = super::probe::encode(key);
                    let url = if key.starts_with("sctp") {
                        let number = super::probe::capture(r"^sctp(\d+)t", key)
                            .map_err(|_| "Server酱 SendKey 格式无效".to_string())?;
                        format!("https://{number}.push.ft07.com/send/{key_encoded}.send")
                    } else {
                        format!("https://sctapi.ftqq.com/{key_encoded}.send")
                    };
                    let channel = settings
                        .serverchan_channel
                        .parse::<i64>()
                        .map_err(|_| "Server酱渠道编号无效".to_string())?;
                    requests.push((url,json!({"title":title,"desp":content,"channel":channel,"tags":settings.serverchan_tags})));
                }
            }
            _ => return Err("未知通知渠道".into()),
        }
        if requests.is_empty() {
            return Err("通知目标未配置".into());
        }
        let proxy = if channel == "Telegram" {
            proxy(settings)
        } else {
            None
        };
        let mut errors = Vec::new();
        for (url, payload) in requests {
            let result = self
                .post(&url, &payload, proxy.clone())
                .await
                .and_then(|reply| Self::acknowledge(channel, &reply));
            if let Err(error) = result {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("；"))
        }
    }
    pub async fn send(
        self: &Arc<Self>,
        settings: Arc<CoreSettings>,
        job: RecordingJob,
        event: NotificationEvent,
        app: Option<AppHandle>,
        cancel: CancellationToken,
    ) -> Vec<String> {
        if cancel.is_cancelled() {
            return Vec::new();
        }
        let (title, content) = message(&settings, &job, event);
        let (desktop, push) = match event {
            NotificationEvent::Start => (
                settings.system_stream_start_notification_enabled,
                settings.stream_start_notification_enabled,
            ),
            NotificationEvent::End => (
                settings.system_stream_end_notification_enabled,
                settings.stream_end_notification_enabled,
            ),
            NotificationEvent::Error => (
                settings.system_error_notification_enabled,
                settings.stream_error_notification_enabled,
            ),
        };
        if settings.system_notification_enabled && desktop {
            if let Some(app) = app {
                crate::desktop::notify(&app, &title, &content);
            }
        }
        if !push || !job.input.enabled_message_push {
            return Vec::new();
        }
        let channels = [
            ("钉钉", settings.dingtalk_enabled),
            ("微信", settings.wechat_enabled),
            ("Bark", settings.bark_enabled),
            ("ntfy", settings.ntfy_enabled),
            ("Telegram", settings.telegram_enabled),
            ("邮件", settings.email_enabled),
            ("Server酱", settings.serverchan_enabled),
        ];
        let mut work = tokio::task::JoinSet::new();
        for (channel, enabled) in channels {
            if !enabled {
                continue;
            }
            let service = self.clone();
            let settings = settings.clone();
            let title = title.clone();
            let content = content.clone();
            let cancel = cancel.clone();
            work.spawn(async move{let result=tokio::select!{_=cancel.cancelled()=>return None,result=tokio::time::timeout(Duration::from_secs(10),service.channel(channel,&settings,&title,&content))=>result.unwrap_or_else(|_|Err("发送超时".into()))};result.err().map(|error|format!("{channel}通知失败：{error}"))});
        }
        let mut errors = Vec::new();
        while let Some(result) = work.join_next().await {
            match result {
                Ok(Some(error)) => errors.push(error),
                Ok(None) => {}
                Err(_) => errors.push("通知任务异常结束".into()),
            }
        }
        errors
    }
}
async fn email(settings: &CoreSettings, title: &str, content: &str) -> Result<(), String> {
    let sender = settings
        .sender_email
        .parse()
        .map_err(|_| "发件人邮箱无效".to_string())?;
    let mut builder = Message::builder()
        .from(Mailbox::new(Some(settings.sender_name.clone()), sender))
        .subject(title)
        .header(ContentType::TEXT_PLAIN);
    let mut recipients = 0;
    for receiver in entries(&settings.recipient_email) {
        builder = builder.to(receiver.parse().map_err(|_| "收件人邮箱无效".to_string())?);
        recipients += 1;
    }
    if recipients == 0 {
        return Err("未配置收件人".into());
    }
    let message = builder
        .body(content.to_owned())
        .map_err(|_| "邮件内容无效".to_string())?;
    let host = if settings.smtp_server.is_empty() {
        "smtp.qq.com"
    } else {
        &settings.smtp_server
    };
    let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(host)
        .map_err(|_| "SMTP 服务配置无效".to_string())?
        .port(465)
        .credentials(Credentials::new(
            settings.email_username.clone(),
            settings.email_password.clone(),
        ))
        .timeout(Some(Duration::from_secs(10)))
        .build();
    transport
        .send(message)
        .await
        .map_err(|_| "SMTP 认证或发送失败".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };
    #[tokio::test]
    async fn http_success_requires_business_ack_and_redacts_message() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            let mut length = 0;
            loop {
                line.clear();
                reader.read_line(&mut line).await.unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    if key.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap();
                    }
                }
            }
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).await.unwrap();
            let body = r#"{"code":401,"message":"fixture-sensitive-token"}"#;
            let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
            let mut stream = reader.into_inner();
            stream.write_all(header.as_bytes()).await.unwrap();
            stream.write_all(body.as_bytes()).await.unwrap();
        });
        let settings = CoreSettings {
            bark_webhook_url: endpoint,
            ..CoreSettings::default()
        };
        let service = NotificationService::default();
        let error = service
            .channel("Bark", &settings, "测试", "内容")
            .await
            .unwrap_err();
        assert!(error.contains("401"));
        assert!(!error.contains("fixture-sensitive-token"));
        server.await.unwrap();
    }
}
