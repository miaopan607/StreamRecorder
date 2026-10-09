use super::super::{
    models::{JobInput, ERROR, MONITORING},
    probe::{encode, query, ProbeInput, ProbeService, StreamData},
    CoreService,
};
use crate::paths::ProjectPaths;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Deserialize)]
struct Reply {
    path: String,
    body: String,
    #[serde(default = "success")]
    status: u16,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body_contains: String,
    #[serde(default)]
    request_contains: String,
    #[serde(default)]
    query_json: BTreeMap<String, Value>,
}
fn success() -> u16 {
    200
}

#[test]
fn compatibility_query_empty_values() {
    assert!(query("https://example.org/live", "id").is_err());
    assert!(query("https://example.org/live?id", "id").is_err());
    assert!(query("https://example.org/live?id=", "id").is_err());
    assert_eq!(
        query("https://example.org/live?id=&id=10001&id=20002", "id").unwrap(),
        "10001"
    );
    assert_eq!(
        query("https://example.org/live?id=%20", "id").unwrap(),
        " "
    );
    assert_eq!(
        query("https://example.org/live?id=%E4%B8%AD%E6%96%87", "id").unwrap(),
        "中文"
    );
    assert!(query("https://example.org/live?ID=10001", "id").is_err());
}
#[derive(Deserialize)]
struct Fixture {
    platform: String,
    url: String,
    live: Vec<Reply>,
    offline: Vec<Reply>,
    expected_live: Value,
    expected_offline: Value,
    #[serde(default)]
    expected_ld: Option<Value>,
}
struct Server {
    address: String,
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let running = Arc::new(AtomicBool::new(true));
        let active = running.clone();
        let handle = thread::spawn(move || {
            while active.load(Ordering::Acquire) {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                if !active.load(Ordering::Acquire) {
                    break;
                }
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line.is_empty() {
                    continue;
                }
                let parts = line.split_whitespace().collect::<Vec<_>>();
                if parts.len() < 2 {
                    continue;
                }
                let method = parts[0].to_owned();
                let target = parts[1].to_owned();
                let path = target.split('?').next().unwrap();
                let mut request_headers = String::new();
                let mut length = 0usize;
                loop {
                    line.clear();
                    if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
                        break;
                    }
                    request_headers.push_str(&line);
                    if let Some((key, value)) = line.split_once(':') {
                        if key.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap();
                        }
                    }
                }
                if length > 1024 * 1024 {
                    continue;
                }
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                let body = String::from_utf8_lossy(&body);
                let reply = replies.iter().find(|reply| {
                    let query_matches = reply.query_json.iter().all(|(expected_key, expected)| {
                        target
                            .split_once('?')
                            .into_iter()
                            .flat_map(|(_, query)| url::form_urlencoded::parse(query.as_bytes()))
                            .any(|(key, value)| {
                                let value = value.into_owned();
                                let actual = serde_json::from_str::<Value>(&value)
                                    .unwrap_or(Value::String(value));
                                key == expected_key.as_str() && &actual == expected
                            })
                    });
                    reply.path == path
                        && query_matches
                        && (reply.body_contains.is_empty() || body.contains(&reply.body_contains))
                        && (reply.request_contains.is_empty()
                            || request_headers.contains(&reply.request_contains))
                });
                let (status, content, headers) = if let Some(reply) = reply {
                    (reply.status, reply.body.as_str(), Some(&reply.headers))
                } else {
                    (404, "fixture route missing", None)
                };
                let bytes = if method == "HEAD" {
                    b"".as_slice()
                } else {
                    content.as_bytes()
                };
                let mut header=format!("HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n",bytes.len());
                if let Some(headers) = headers {
                    for (name, value) in headers {
                        header.push_str(&format!("{name}: {value}\r\n"));
                    }
                }
                header.push_str("\r\n");
                let _ = stream
                    .write_all(header.as_bytes())
                    .and_then(|_| stream.write_all(bytes));
            }
        });
        Self {
            address,
            running,
            thread: Some(handle),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        let _ = TcpStream::connect(&self.address);
        if let Some(handle) = self.thread.take() {
            handle.join().unwrap();
        }
    }
}
fn compare(platform: &str, actual: &Value, expected: &Value) {
    for (key, value) in expected.as_object().unwrap() {
        if let Some(field) = key.strip_suffix("_prefix") {
            assert!(
                actual[field]
                    .as_str()
                    .is_some_and(|s| s.starts_with(value.as_str().unwrap())),
                "{platform} {key}: {actual}"
            );
        } else if let Some(field) = key.strip_suffix("_suffix") {
            assert!(
                actual[field]
                    .as_str()
                    .is_some_and(|s| s.ends_with(value.as_str().unwrap())),
                "{platform} {key}: {actual}"
            );
        } else {
            assert_eq!(&actual[key], value, "{platform} {key}");
        }
    }
}
const CORPORA: [&str; 5] = [
    include_str!("fixtures/platforms.json"),
    include_str!("fixtures/domestic.json"),
    include_str!("fixtures/international.json"),
    include_str!("fixtures/accounts.json"),
    include_str!("fixtures/signed.json"),
];
#[tokio::test]
async fn all_platform_live_offline_and_quality_contracts() {
    for corpus in CORPORA {
        for fixture in serde_json::from_str::<Vec<Fixture>>(corpus).unwrap() {
            let mut cases = vec![
                (fixture.live.clone(), fixture.expected_live.clone(), "OD"),
                (fixture.offline, fixture.expected_offline, "OD"),
            ];
            if let Some(expected) = fixture.expected_ld {
                cases.push((fixture.live, expected, "LD"));
            }
            for (replies, expected, quality) in cases {
                let server = Server::new(replies);
                let service = ProbeService::default();
                *service.endpoint.lock() = Some(format!("http://{}", server.address));
                let input = ProbeInput {
                    platform_key: fixture.platform.clone(),
                    live_url: fixture.url.clone(),
                    quality: quality.into(),
                    ..ProbeInput::default()
                };
                let result = tokio::time::timeout(
                    Duration::from_secs(5),
                    service.probe(input, CancellationToken::new()),
                )
                .await
                .expect("平台 fixture 超时")
                .unwrap_or_else(|error| panic!("{} {quality}: {error}", fixture.platform));
                let actual = serde_json::to_value(result).unwrap();
                compare(&fixture.platform, &actual, &expected);
            }
        }
    }
}
#[tokio::test]
async fn http_failure_is_not_reported_as_offline() {
    let server = Server::new(vec![Reply {
        status: 403,
        ..reply("/langweb/v1/room/liveinfo", "denied")
    }]);
    let service = ProbeService::default();
    *service.endpoint.lock() = Some(format!("http://{}", server.address));
    let error = service
        .probe(
            ProbeInput {
                platform_key: "lang".into(),
                live_url: "https://www.lang.live/10001".into(),
                quality: "OD".into(),
                ..ProbeInput::default()
            },
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(error.contains("HTTP 状态 403"));
}

#[tokio::test]
async fn malformed_responses_are_not_offline() {
    for corpus in CORPORA {
        for fixture in serde_json::from_str::<Vec<Fixture>>(corpus).unwrap() {
            let replies = fixture
                .live
                .into_iter()
                .map(|mut reply| {
                    reply.body = "{\"unexpected\":true}".into();
                    reply
                })
                .collect();
            let server = Server::new(replies);
            let service = ProbeService::default();
            *service.endpoint.lock() = Some(format!("http://{}", server.address));
            let result = service
                .probe(
                    ProbeInput {
                        platform_key: fixture.platform.clone(),
                        live_url: fixture.url,
                        quality: "OD".into(),
                        ..ProbeInput::default()
                    },
                    CancellationToken::new(),
                )
                .await;
            assert!(
                result.is_err(),
                "{} 把畸形响应作为正常结果：{result:?}",
                fixture.platform
            );
        }
    }
}

fn reply(path: &str, body: &str) -> Reply {
    Reply {
        path: path.into(),
        body: body.into(),
        status: 200,
        headers: BTreeMap::new(),
        body_contains: String::new(),
        request_contains: String::new(),
        query_json: BTreeMap::new(),
    }
}
fn account_fixture(key: &str) -> Fixture {
    serde_json::from_str::<Vec<Fixture>>(CORPORA[3])
        .unwrap()
        .into_iter()
        .find(|fixture| fixture.platform == key)
        .unwrap()
}
fn next_page(value: Value) -> String {
    format!("<script id=\"__NEXT_DATA__\" type=\"application/json\">{value}</script>")
}

fn rednote_page(id: &str, nickname: Option<&str>) -> String {
    let flv = format!("http://live-source-play.xhscdn.com/live/{id}.flv");
    let mut deeplink = format!("xhsdiscover://live?flvUrl={}", encode(&flv));
    if let Some(nickname) = nickname {
        deeplink.push_str("&host_nickname=");
        deeplink.push_str(&encode(nickname));
    }
    let state = serde_json::json!({
        "liveStream": {
            "liveStatus": "success",
            "roomData": {
                "roomInfo": {
                    "roomTitle": "直播标题",
                    "deeplink": deeplink
                }
            }
        }
    });
    format!("<script>window.__INITIAL_STATE__={state}</script>")
}

async fn run_probe(
    platform: &str,
    live_url: &str,
    routes: Vec<Reply>,
) -> Result<StreamData, String> {
    let server = Server::new(routes);
    let service = ProbeService::default();
    *service.endpoint.lock() = Some(format!("http://{}", server.address));
    service
        .probe(
            ProbeInput {
                platform_key: platform.into(),
                live_url: live_url.into(),
                quality: "OD".into(),
                ..ProbeInput::default()
            },
            CancellationToken::new(),
        )
        .await
}

#[tokio::test]
async fn compatibility_offline_netease_missing_nicknames() {
    let body = format!(
        "<script id=\"__NEXT_DATA__\" type=\"application/json\" crossorigin=\"anonymous\">{}</script></body>",
        serde_json::json!({"props":{"pageProps":{"roomInfoInitData":{"live":{"status":0}}}}})
    );
    let data = run_probe(
        "netease",
        "https://cc.163.com/10001",
        vec![reply("/10001/", &body)],
    ).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name, None);
    assert_eq!(data.record_url, None);

    for live in [serde_json::json!({}), serde_json::json!({"status":null}), serde_json::json!({"status":"unknown"})] {
        let body = format!(
            "<script id=\"__NEXT_DATA__\" type=\"application/json\" crossorigin=\"anonymous\">{}</script></body>",
            serde_json::json!({"props":{"pageProps":{"roomInfoInitData":{"live":live}}}})
        );
        assert!(run_probe(
            "netease", "https://cc.163.com/10001", vec![reply("/10001/", &body)],
        ).await.is_err());
    }
}

#[tokio::test]
async fn compatibility_offline_sixroom_null_stream_title() {
    let body = serde_json::json!({
        "content": {"liveinfo":{"flvtitle":null},"roominfo":{"alias":"当前主播"}}
    }).to_string();
    let routes = vec![
        reply("/10001", "rid: '10001',\n roomid"),
        reply("/coop/mobile/index.php", &body),
    ];
    let data = run_probe("6room", "https://v.6.cn/10001", routes).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("当前主播"));
    assert_eq!(data.record_url, None);

    let body = serde_json::json!({"content":{"liveinfo":{},"roominfo":{"alias":"当前主播"}}}).to_string();
    assert!(run_probe("6room", "https://v.6.cn/10001", vec![
        reply("/10001", "rid: '10001',\n roomid"),
        reply("/coop/mobile/index.php", &body),
    ]).await.is_err());
}

#[tokio::test]
async fn compatibility_offline_vvxq_null_name_uses_secondary_profile() {
    let routes = vec![
        reply("/room/video/getRoomData.do", r#"{"status":0}"#),
        reply("/activity-center/fanclub/activity/captain/banner", r#"{"data":{"anchorName":null}}"#),
        reply("/activity-center/halloween2023/banner", r#"{"data":{"memberVO":{"memberName":"回退主播"}}}"#),
    ];
    let data = run_probe(
        "vvxq", "https://h5webcdn-pro.vvxqiu.com/?roomId=10001", routes,
    ).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("回退主播"));
    assert_eq!(data.record_url, None);

    assert!(run_probe(
        "vvxq", "https://h5webcdn-pro.vvxqiu.com/?roomId=10001", vec![
            reply("/room/video/getRoomData.do", r#"{"status":0}"#),
            reply("/activity-center/fanclub/activity/captain/banner", r#"{"data":{}}"#),
            reply("/activity-center/halloween2023/banner", r#"{"data":{"memberVO":{"memberName":"回退主播"}}}"#),
        ],
    ).await.is_err());
}

#[tokio::test]
async fn compatibility_offline_bigo_null_nickname() {
    let data = run_probe(
        "bigo", "https://www.bigo.tv/10001", vec![reply(
            "/official_website/studio/getInternalStudioInfo", r#"{"data":{"alive":0,"nick_name":null}}"#,
        )],
    ).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name, None);
    assert_eq!(data.record_url, None);
    // 显式空字符串仍应走旧主页昵称回退；null 不应触发这条请求。
    let data = run_probe(
        "bigo", "https://www.bigo.tv/sg/10001", vec![
            reply("/official_website/studio/getInternalStudioInfo", r#"{"data":{"alive":0,"nick_name":""}}"#),
            reply("/sg/10001", "<title>欢迎来到主页主播的直播间</title>"),
        ],
    ).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("主页主播"));

    assert!(run_probe(
        "bigo", "https://www.bigo.tv/10001", vec![reply(
            "/official_website/studio/getInternalStudioInfo", r#"{"data":{"alive":0}}"#,
        )],
    ).await.is_err());
}

#[tokio::test]
async fn compatibility_offline_showroom_null_room_name() {
    let data = run_probe(
        "showroom", "https://www.showroom-live.com/room/profile?room_id=10001", vec![reply(
            "/api/live/live_info", r#"{"live_status":0,"room_name":null}"#,
        )],
    ).await.unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name, None);
    assert_eq!(data.record_url, None);

    assert!(run_probe(
        "showroom", "https://www.showroom-live.com/room/profile?room_id=10001", vec![reply(
            "/api/live/live_info", r#"{"live_status":0}"#,
        )],
    ).await.is_err());
}

fn taobao_routes(expected_id: &str) -> Vec<Reply> {
    let fixture = serde_json::from_str::<Vec<Fixture>>(CORPORA[4])
        .unwrap()
        .into_iter()
        .find(|fixture| fixture.platform == "taobao")
        .unwrap();
    let mut api = fixture
        .live
        .into_iter()
        .find(|route| route.path == "/h5/mtop.mediaplatform.live.livedetail/4.0/")
        .unwrap();
    api.query_json.insert(
        "data".into(),
        serde_json::json!({"liveId":expected_id,"creatorId":null}),
    );
    vec![api]
}

#[tokio::test]
async fn compatibility_taobao_empty_id() {
    for url in [
        "https://tb.cn/live?id=&liveId=10001",
        "https://tb.cn/live?id=10001&liveId=20002",
    ] {
        let data = run_probe("taobao", url, taobao_routes("10001"))
            .await
            .unwrap();
        assert!(data.is_live);
        assert_eq!(
            data.record_url.as_deref(),
            Some("https://media.example/high.m3u8")
        );
    }

    let page = reply(
        "/legacy",
        "var url = 'https://tbzb.taobao.com/live?id=&liveId=10001';",
    );
    let mut routes = vec![page];
    routes.extend(taobao_routes("10001"));
    let data = run_probe(
        "taobao",
        "https://tb.cn/legacy?id=&liveId=",
        routes,
    )
    .await
    .unwrap();
    assert!(data.is_live);
    assert_eq!(
        data.record_url.as_deref(),
        Some("https://media.example/high.m3u8")
    );
}

#[tokio::test]
async fn compatibility_jd_empty_author_id_uses_room_fragment() {
    let mut redirect = reply("/s/10001", "");
    redirect.status = 302;
    redirect.headers.insert(
        "Location".into(),
        "https://lives.jd.com/live?authorId=#/10001?origin=fixture".into(),
    );
    let mut play = reply(
        "/client.action",
        r#"{"data":{"status":1,"videoUrl":"https://media.example/live.flv","h5VideoUrl":"https://media.example/master.m3u8"}}"#,
    );
    play.query_json
        .insert("body".into(), serde_json::json!({"liveId":"10001"}));
    let data = run_probe(
        "jd",
        "https://3.cn/s/10001",
        vec![redirect, reply("/live", ""), play],
    )
    .await
    .unwrap();
    assert!(data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("jd_10001"));
    assert_eq!(
        data.record_url.as_deref(),
        Some("https://media.example/master.m3u8")
    );
}

#[tokio::test]
async fn compatibility_rednote_legacy_links() {
    let data = run_probe(
        "xiaohongshu",
        "https://www.xiaohongshu.com/livestream/10001",
        vec![reply(
            "/livestream/10001",
            &rednote_page("10001", Some("主播")),
        )],
    )
    .await
    .unwrap();
    assert!(data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("主播"));
    assert_eq!(
        data.record_url.as_deref(),
        Some("http://live-source-play.xhscdn.com/live/10001.flv")
    );

    let mut short_link = reply("/share/10002", "");
    short_link.status = 302;
    short_link.headers.insert(
        "Location".into(),
        "https://www.xiaohongshu.com/livestream/10002".into(),
    );
    let data = run_probe(
        "xhs",
        "https://xhslink.com/share/10002",
        vec![
            short_link,
            reply("/livestream/10002", &rednote_page("10002", None)),
        ],
    )
    .await
    .unwrap();
    assert!(data.is_live);
    assert_eq!(data.anchor_name, None);
    assert_eq!(
        data.live_url.as_deref(),
        Some("https://www.xiaohongshu.com/livestream/10002")
    );
    assert_eq!(
        data.record_url.as_deref(),
        Some("http://live-source-play.xhscdn.com/live/10002.flv")
    );

    let data = run_probe(
        "xiaohongshu",
        "https://www.xiaohongshu.com/user/profile/10003",
        vec![reply(
            "/user/profile/10003",
            &rednote_page("10003", Some("")),
        )],
    )
    .await
    .unwrap();
    assert!(data.is_live);
    assert_eq!(data.anchor_name, None);
    assert_eq!(
        data.record_url.as_deref(),
        Some("http://live-source-play.xhscdn.com/live/10003.flv")
    );
}

#[tokio::test]
async fn compatibility_rednote_ended_room_does_not_follow_recommendation() {
    // 真实直播页的下播结构同时带有推荐直播，不能把推荐房间作为当前任务。
    let state = serde_json::json!({
        "liveStream": {
            "pageStatus": "success",
            "liveStatus": "end",
            "roomData": {
                "hostInfo": {"nickName": "当前主播"},
                "roomInfo": {"status": 3, "roomTitle": "当前直播已结束"}
            },
            "nextRoomInfo": {
                "nickName": "推荐主播",
                "roomId": "20004",
                "deeplink": "xhsdiscover://live?host_nickname=推荐主播&flvUrl=http%3A%2F%2Flive-source-play.xhscdn.com%2Flive%2F20004.flv"
            }
        }
    });
    let body = format!("<script>window.__INITIAL_STATE__={state}</script>");
    let data = run_probe(
        "xiaohongshu",
        "https://www.xiaohongshu.com/livestream/10004",
        vec![reply("/livestream/10004", &body)],
    )
    .await
    .unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("当前主播"));
    assert_eq!(data.title.as_deref(), Some("当前直播已结束"));
    assert!(data.record_url.is_none());
    assert!(data.m3u8_url.is_none());
    assert!(data.flv_url.is_none());

    for pointer in ["/liveStream/pageStatus", "/liveStream/roomData/roomInfo/status"] {
        let mut invalid = state.clone();
        *invalid.pointer_mut(pointer).unwrap() = Value::Null;
        let body = format!("<script>window.__INITIAL_STATE__={invalid}</script>");
        assert!(run_probe(
            "xiaohongshu",
            "https://www.xiaohongshu.com/livestream/10004",
            vec![reply("/livestream/10004", &body)],
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn compatibility_rednote_invalid_pages_remain_errors() {
    for body in [
        "<html>not a RedNote room</html>".into(),
        "<script>window.__INITIAL_STATE__={broken}</script>".into(),
        format!(
            "<script>window.__INITIAL_STATE__={}</script>",
            serde_json::json!({"liveStream":{"liveStatus":"success","roomData":{"roomInfo":{"roomTitle":"直播标题","deeplink":"xhsdiscover://live?host_nickname=主播"}}}})
        ),
    ] {
        assert!(run_probe(
            "xiaohongshu",
            "https://www.xiaohongshu.com/livestream/without-host-id",
            vec![reply("/livestream/without-host-id", &body)],
        )
        .await
        .is_err());
    }

    let data = run_probe(
        "xiaohongshu",
        "https://www.xiaohongshu.com/user/profile/10003",
        vec![reply(
            "/user/profile/10003",
            "<title>@主播 的个人主页</title>",
        )],
    )
    .await
    .unwrap();
    assert!(!data.is_live);
    assert_eq!(data.anchor_name.as_deref(), Some("主播"));
}

#[tokio::test]
async fn compatibility_rednote_preserves_configured_name() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::new(vec![reply(
        "/livestream/10002",
        &rednote_page("10002", None),
    )]);
    let core = CoreService::new(ProjectPaths::at(directory.path().to_owned()), None);
    *core.probe.endpoint.lock() = Some(format!("http://{}", server.address));
    core.start().await.unwrap();
    let mut settings = core.snapshot().settings;
    settings.only_notify_no_record = true;
    settings.system_notification_enabled = false;
    core.settings_update(settings).await.unwrap();
    let id = core
        .jobs_upsert(vec![JobInput {
            url: "https://www.xiaohongshu.com/livestream/10002".into(),
            streamer_name: "原配置主播".into(),
            ..JobInput::default()
        }])
        .await
        .unwrap()
        .changed_ids[0]
        .clone();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let job = core.job(&id).unwrap();
            assert_ne!(job.status_info, ERROR, "{}", job.error_message);
            if job.status_info == MONITORING {
                assert_eq!(job.input.streamer_name, "原配置主播");
                assert_eq!(job.live_title, "直播标题");
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    core.shutdown().await;
}
#[tokio::test]
async fn failed_logins_are_not_reported_as_offline() {
    for key in ["flextv", "soop", "popkontv", "twitcasting"] {
        let fixture = account_fixture(key);
        let mut url = fixture.url;
        let mut routes = fixture.live;
        match key {
            "flextv" => {
                routes[0].body = next_page(
                    serde_json::json!({"props":{"pageProps":{"channelStream":{"channel":{"message":"로그인후 이용이 가능합니다."}}}}}),
                );
                routes.push(reply(
                    "/v2/api/auth/signin",
                    "{\"error\":\"invalid credentials\"}",
                ));
            }
            "soop" => {
                routes
                    .iter_mut()
                    .filter(|r| r.path == "/afreeca/player_live_api.php")
                    .for_each(|r| r.body = "{\"CHANNEL\":{\"RESULT\":-5}}".into());
                routes.push(reply("/app/LoginAction.php", "{\"RESULT\":-1}"));
            }
            "popkontv" => {
                routes
                    .iter_mut()
                    .filter(|r| r.path.contains("castwatchonoffguest"))
                    .for_each(|r| r.body = "{\"statusCd\":\"E5000\"}".into());
                routes.push(reply(
                    "/api/proxy/member/v1/login",
                    "{\"statusCd\":\"E1000\"}",
                ));
            }
            "twitcasting" => {
                url.push_str("?login=true");
                let mut failure = reply("/indexcaslogin.php", "invalid credentials");
                failure.body_contains = "action=login".into();
                routes.push(failure);
                routes.push(reply(
                    "/indexcaslogin.php",
                    "<input type=\"hidden\" name=\"cs_session_id\" value=\"fixture-session\">",
                ));
            }
            _ => unreachable!(),
        }
        let server = Server::new(routes);
        let service = ProbeService::default();
        *service.endpoint.lock() = Some(format!("http://{}", server.address));
        let error = service
            .probe(
                ProbeInput {
                    platform_key: key.into(),
                    live_url: url,
                    quality: "OD".into(),
                    username: Some("fixture_user".into()),
                    password: Some("fixture_password".into()),
                    ..ProbeInput::default()
                },
                CancellationToken::new(),
            )
            .await
            .unwrap_err();
        assert!(error.contains("登录失败"), "{key}: {error}");
    }
}
#[tokio::test]
async fn successful_login_updates_cookie_and_unlocks_stream() {
    let fixture = account_fixture("flextv");
    let mut routes = fixture.live;
    let mut authenticated = reply(
        "/channels/user/live",
        &next_page(
            serde_json::json!({"props":{"pageProps":{"channel":{"owner":{"nickname":"主播","loginId":"user"}}}}}),
        ),
    );
    authenticated.request_contains = "flx_oauth_access=fixture-access".into();
    routes[0].body = next_page(
        serde_json::json!({"props":{"pageProps":{"channelStream":{"channel":{"message":"로그인후 이용이 가능합니다."}}}}}),
    );
    routes.insert(0, authenticated);
    let mut login = reply("/v2/api/auth/signin", "{}");
    login.headers.insert(
        "Set-Cookie".into(),
        "flx_oauth_access=fixture-access; Path=/; HttpOnly".into(),
    );
    routes.push(login);
    let server = Server::new(routes);
    let service = ProbeService::default();
    *service.endpoint.lock() = Some(format!("http://{}", server.address));
    let input = ProbeInput {
        platform_key: "flextv".into(),
        live_url: fixture.url,
        quality: "OD".into(),
        username: Some("fixture_user".into()),
        password: Some("fixture_password".into()),
        ..ProbeInput::default()
    };
    let data = service
        .probe(input, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        data.new_cookies.as_deref(),
        Some("flx_oauth_access=fixture-access")
    );
    assert_eq!(
        data.record_url.as_deref(),
        Some("https://media.example/high.m3u8")
    );
}

#[tokio::test]
async fn douyin_falls_back_to_real_app_response() {
    let fixture = serde_json::from_str::<Vec<Fixture>>(CORPORA[4])
        .unwrap()
        .into_iter()
        .find(|f| f.platform == "douyin")
        .unwrap();
    let web: Value = serde_json::from_str(&fixture.live[0].body).unwrap();
    let mut room = web["data"]["data"][0].clone();
    room["owner"] = serde_json::json!({"nickname":"主播","web_rid":"10001"});
    let mut redirect = reply("/10001", "");
    redirect.status = 302;
    redirect.headers.insert(
        "Location".into(),
        "https://webcast.amemv.com/reflow/room/10001?sec_user_id=fixture".into(),
    );
    let routes = vec![
        Reply {
            status: 403,
            ..reply("/webcast/room/web/enter/", "denied")
        },
        redirect,
        reply("/reflow/room/10001", ""),
        reply(
            "/webcast/room/reflow/info/",
            &serde_json::json!({"data":{"room":room}}).to_string(),
        ),
        reply("/high.m3u8&codec=h264", ""),
    ];
    let server = Server::new(routes);
    let service = ProbeService::default();
    *service.endpoint.lock() = Some(format!("http://{}", server.address));
    let data = service
        .probe(
            ProbeInput {
                platform_key: "douyin".into(),
                live_url: fixture.url,
                quality: "OD".into(),
                ..ProbeInput::default()
            },
            CancellationToken::new(),
        )
        .await
        .unwrap();
    compare(
        "douyin app",
        &serde_json::to_value(data).unwrap(),
        &fixture.expected_live,
    );
}
#[tokio::test]
async fn cancellation_closes_pending_http_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted, connected) = tokio::sync::oneshot::channel();
    let peer = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = [0; 4096];
        assert!(stream.read(&mut bytes).unwrap() > 0);
        accepted.send(()).unwrap();
        loop {
            match stream.read(&mut bytes) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) => panic!("取消后连接仍未关闭：{error}"),
            }
        }
    });
    let service = Arc::new(ProbeService::default());
    *service.endpoint.lock() = Some(format!("http://{address}"));
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let probe_service = service.clone();
    let pending = tokio::spawn(async move {
        probe_service
            .probe(
                ProbeInput {
                    platform_key: "lang".into(),
                    live_url: "https://www.lang.live/10001".into(),
                    quality: "OD".into(),
                    ..ProbeInput::default()
                },
                token,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(1), connected)
        .await
        .unwrap()
        .unwrap();
    cancel.cancel();
    let result = tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
    assert!(result.unwrap_err().contains("探测已取消"));
    peer.join().unwrap();
}
