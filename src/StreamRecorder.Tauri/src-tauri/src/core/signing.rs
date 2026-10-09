use base64::{engine::general_purpose::STANDARD, Engine as _};
use md5::{Digest, Md5};
use rquickjs::{function::Args, Context, Function, Object, Runtime};
use serde_json::Value;
use std::{
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub const STREAMGET_LICENSE: &str = include_str!("assets/STREAMGET-LICENSE");
static SIGNERS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));
pub fn md5(text: impl AsRef<[u8]>) -> String {
    format!("{:x}", Md5::digest(text.as_ref()))
}
pub fn base64(text: impl AsRef<[u8]>) -> String {
    STANDARD.encode(text.as_ref())
}

pub fn rc4(input: &[u8], key: &[u8]) -> Vec<u8> {
    let mut state = std::array::from_fn::<_, 256, _>(|i| i as u8);
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + state[i] as usize + key[i % key.len()] as usize) & 255;
        state.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    input
        .iter()
        .map(|byte| {
            i = (i + 1) & 255;
            j = (j + state[i] as usize) & 255;
            state.swap(i, j);
            *byte ^ state[(state[i] as usize + state[j] as usize) & 255]
        })
        .collect()
}
fn custom_base64(input: &[u8], table: &[u8; 64]) -> String {
    let mut result = String::with_capacity((input.len() * 4).div_ceil(3));
    for chunk in input.chunks(3) {
        let value = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        for shift in [18, 12, 6, 0]
            .into_iter()
            .take((chunk.len() * 4).div_ceil(3))
        {
            result.push(table[((value >> shift) & 63) as usize] as char);
        }
    }
    result
}
// SM3/RC4 与字节布局来自固定版本 a_bogus，时间可注入以校对源向量。
pub fn douyin_ab(query: &str, user_agent: &str, start: u64) -> String {
    use sm3::Sm3;
    let mut hash = Sm3::new();
    hash.update(query.as_bytes());
    hash.update(b"cus");
    let query_hash = Sm3::digest(hash.finalize());
    let suffix_hash = Sm3::digest(Sm3::digest(b"cus"));
    let ua_hash = Sm3::digest(
        custom_base64(
            &rc4(user_agent.as_bytes(), &[0, 1, 14]),
            b"ckdp1h4ZKsUB80/Mfvw36XIgR25+WQAlEi7NLboqYTOPuzmFjJnryx9HVGDaStCe",
        )
        .as_bytes(),
    );
    let end = start + 100;
    let mut b = [0u8; 73];
    b[18] = 44;
    b[20..24].copy_from_slice(&(start as u32).to_be_bytes());
    b[24] = (start >> 32) as u8;
    b[25] = (start >> 40) as u8;
    b[31] = 1;
    b[37] = 14;
    b[38] = query_hash[21];
    b[39] = query_hash[22];
    b[40] = suffix_hash[21];
    b[41] = suffix_hash[22];
    b[42] = ua_hash[23];
    b[43] = ua_hash[24];
    b[44..48].copy_from_slice(&(end as u32).to_be_bytes());
    b[48] = 3;
    b[49] = (end >> 32) as u8;
    b[50] = (end >> 40) as u8;
    b[52..56].copy_from_slice(&110624u32.to_be_bytes());
    b[57..61].copy_from_slice(&6383u32.to_le_bytes());
    let window = b"1920|1080|1920|1040|0|30|0|0|1872|92|1920|1040|1857|92|1|24|Win32";
    b[65] = window.len() as u8;
    b[66] = (window.len() >> 8) as u8;
    for index in [
        18, 20, 26, 30, 38, 40, 42, 21, 27, 31, 35, 39, 41, 43, 22, 28, 32, 36, 23, 29, 33, 37, 44,
        45, 46, 47, 48, 49, 50, 24, 25, 52, 53, 54, 55, 57, 58, 59, 60, 65, 66, 70, 71,
    ] {
        b[72] ^= b[index];
    }
    let mut payload = Vec::with_capacity(45 + window.len());
    for index in [
        18, 20, 52, 26, 30, 34, 58, 38, 40, 53, 42, 21, 27, 54, 55, 31, 35, 57, 39, 41, 43, 22, 28,
        32, 60, 36, 23, 29, 33, 37, 44, 45, 59, 46, 47, 48, 49, 50, 24, 25, 65, 66, 70, 71,
    ] {
        payload.push(b[index]);
    }
    payload.extend_from_slice(window);
    payload.push(b[72]);
    let mut complete = Vec::with_capacity(12 + payload.len());
    for (number, options) in [(1234u16, [3, 45]), (9876, [1, 0]), (5555, [1, 5])] {
        let lo = number as u8;
        let hi = (number >> 8) as u8;
        complete.extend_from_slice(&[
            (lo & 170) | (options[0] & 85),
            (lo & 85) | (options[0] & 170),
            (hi & 170) | (options[1] & 85),
            (hi & 85) | (options[1] & 170),
        ]);
    }
    complete.extend_from_slice(&rc4(&payload, &[121]));
    let mut result = custom_base64(
        &complete,
        b"Dkdpgh2ZmsQB80/MfvV36XI1R45-WUAlEixNLwoqYTOPuzKFjJnry79HbGcaStCe",
    );
    result.push('=');
    result
}
pub fn look_encrypt(payload: &str, key: &[u8; 16]) -> (String, String) {
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    use num_bigint::BigUint;
    static MODULUS: LazyLock<BigUint> = LazyLock::new(|| {
        BigUint::parse_bytes(b"00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7", 16).expect("固定 RSA 模数有效")
    });
    let encrypt = |value: &[u8], key: &[u8; 16]| {
        cbc::Encryptor::<aes::Aes128>::new(key.into(), b"0102030405060708".into())
            .encrypt_padded_vec_mut::<Pkcs7>(value)
    };
    let first = base64(encrypt(payload.as_bytes(), b"0CoJUm6Qyw8W8jud"));
    let params = base64(encrypt(first.as_bytes(), key));
    let mut reversed = *key;
    reversed.reverse();
    let encrypted = BigUint::from_bytes_be(&reversed).modpow(&BigUint::from(65537u32), &MODULUS);
    (params, format!("{encrypted:0>256x}"))
}

// 只执行内嵌签名资源；参数经 JSON 值转换，不参与脚本拼接。
pub async fn sign(
    kind: &str,
    args: Vec<Value>,
    cancel: CancellationToken,
) -> Result<Value, String> {
    let source = match kind {
        "xbogus" => include_str!("assets/x-bogus.js"),
        "haixiu" => include_str!("assets/haixiu.js"),
        "liveme" => include_str!("assets/liveme.js"),
        _ => return Err("未知签名算法".into()),
    };
    let slots = SIGNERS.clone();
    let permit = tokio::select! { _ = cancel.cancelled() => return Err("探测已取消".into()), value = slots.acquire_owned() => value.map_err(|e| e.to_string())? };
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        run_javascript(source, "sign", &args, cancel, Duration::from_secs(2))
    })
    .await
    .map_err(|e| format!("签名任务失败：{e}"))?
}
fn run_javascript(
    source: &str,
    function: &str,
    values: &[Value],
    cancel: CancellationToken,
    limit: Duration,
) -> Result<Value, String> {
    let runtime = Runtime::new().map_err(|e| e.to_string())?;
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let expires = Instant::now() + limit;
    let interrupted = cancel.clone();
    runtime.set_interrupt_handler(Some(Box::new(move || {
        interrupted.is_cancelled() || Instant::now() >= expires
    })));
    let context = Context::full(&runtime).map_err(|e| e.to_string())?;
    context.with(|ctx| {
        let execute = || -> rquickjs::Result<String> {
            ctx.eval::<(), _>(r#"var module={exports:{}}; var exports=module.exports;
                var console={log:function(){},error:function(){},warn:function(){}};
                function atob(text){var chars='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/',out='',bits=0,count=0; text=String(text).replace(/=+$/,''); for(var i=0;i<text.length;i++){var value=chars.indexOf(text[i]); if(value<0)throw new Error('Invalid base64');bits=(bits<<6)|value;count+=6;if(count>=8){count-=8;out+=String.fromCharCode((bits>>count)&255);}}return out;}
            "#)?;
            ctx.eval::<(), _>(include_str!("assets/crypto-js.min.js"))?;
            ctx.eval::<(), _>("var __crypto=module.exports; module={exports:{}}; exports=module.exports; function require(name){if(name==='crypto-js.min.js')return __crypto; throw new Error('Module is not embedded');}")?;
            // 上游 CommonJS 脚本使用非严格模式的全局赋值，保持原有执行语义。
            let mut options = rquickjs::context::EvalOptions::default();
            options.strict = false;
            ctx.eval_with_options::<(), _>(source, options)?;
            let module: Object = ctx.globals().get("module")?;
            let exports: Object = module.get("exports")?;
            let call: Function = exports.get(function)?;
            let mut args = Args::new(ctx.clone(), values.len());
            for value in values { args.push_arg(ctx.json_parse(serde_json::to_vec(value).expect("JSON 值可序列化"))?)?; }
            let result: rquickjs::Value = args.apply(&call)?;
            ctx.json_stringify(result)?.ok_or(rquickjs::Error::Unknown)?.to_string()
        };
        match execute() {
            Ok(text) => serde_json::from_str(&text).map_err(|e| e.to_string()),
            Err(_) if cancel.is_cancelled() => Err("探测已取消".into()),
            Err(_) if Instant::now() >= expires => Err("签名计算超时".into()),
            // 不返回可能包含凭据和参数的 JS exception 文本。
            Err(_) => Err("签名计算失败或资源限制已触发".into()),
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_signatures_match_independent_vectors() {
        let fixture: Value =
            serde_json::from_str(include_str!("platforms/fixtures/signatures.json")).unwrap();
        let vector = &fixture["douyin"];
        assert_eq!(
            douyin_ab(
                vector["query"].as_str().unwrap(),
                vector["user_agent"].as_str().unwrap(),
                vector["timestamp"].as_u64().unwrap()
            ),
            vector["signature"]
        );
        assert_eq!(
            rc4(b"Plaintext", b"Key"),
            vec![0xbb, 0xf3, 0x16, 0xe8, 0xd9, 0x40, 0xaf, 0x0a, 0xd3]
        );
        let payload = "{\"liveRoomNo\": \"10001\"}";
        let (cipher, secret) = look_encrypt(payload, b"abcdefghijklmnop");
        let rsa: Value =
            serde_json::from_str(include_str!("platforms/fixtures/look-rsa.json")).unwrap();
        assert_eq!(cipher, rsa["encrypted_payload"]);
        assert_eq!(secret, rsa["encrypted_key"]);
    }
    #[test]
    fn embedded_signatures_match_upstream_vectors() {
        let vectors: Vec<Value> = serde_json::from_str(include_str!(
            "platforms/fixtures/javascript-signatures.json"
        ))
        .unwrap();
        let hook="Date=class extends Date{constructor(...a){super(...(a.length?a:[1760000000123]))}static now(){return 1760000000123}};Math.random=function(){return .25};";
        for vector in vectors {
            let source = match vector["kind"].as_str().unwrap() {
                "xbogus" => include_str!("assets/x-bogus.js"),
                "haixiu" => include_str!("assets/haixiu.js"),
                "liveme" => include_str!("assets/liveme.js"),
                _ => unreachable!(),
            };
            let actual = run_javascript(
                &format!("{hook}{source}"),
                "sign",
                vector["args"].as_array().unwrap(),
                CancellationToken::new(),
                Duration::from_secs(2),
            )
            .unwrap();
            assert_eq!(actual, vector["expected"], "{}", vector["kind"]);
        }
    }
    #[test]
    fn javascript_is_bounded_and_recovers() {
        let cancel = CancellationToken::new();
        let started = Instant::now();
        assert!(run_javascript(
            "module.exports.sign=function(){while(true){}}",
            "sign",
            &[],
            cancel.clone(),
            Duration::from_millis(50)
        )
        .is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
        for source in [
            "module.exports.sign=function recurse(){return recurse()}",
            "module.exports.sign=function(){var arrays=[];while(true)arrays.push(new Uint8Array(1024*1024))}",
        ] {
            let started=Instant::now();
            assert!(run_javascript(source,"sign",&[],cancel.clone(),Duration::from_secs(2)).is_err());
            assert!(started.elapsed()<Duration::from_secs(3));
        }
        let value = run_javascript("module.exports.sign=function(value){return {value:value, hash:require('crypto-js.min.js').MD5(value).toString()}}", "sign", &[Value::String("中文🎬".into())], cancel, Duration::from_secs(2)).unwrap();
        assert_eq!(value["value"], "中文🎬");
        assert_eq!(value["hash"], md5("中文🎬"));
    }
    #[test]
    fn wasm_fuel_interrupts_computation() {
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = wasmi::Engine::new(&config);
        let module =
            wasmi::Module::new(&engine, "(module (func (export \"run\") (loop br 0)))").unwrap();
        let mut store = wasmi::Store::new(
            &engine,
            wasmi::StoreLimitsBuilder::new()
                .memory_size(64 * 1024 * 1024)
                .build(),
        );
        store.limiter(|limits| limits);
        store.set_fuel(1000).unwrap();
        let instance = wasmi::Linker::new(&engine)
            .instantiate_and_start(&mut store, &module)
            .unwrap();
        assert!(instance
            .get_typed_func::<(), ()>(&store, "run")
            .unwrap()
            .call(&mut store, ())
            .is_err());
    }
}
