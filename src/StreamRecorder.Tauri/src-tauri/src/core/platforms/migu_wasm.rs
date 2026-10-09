use crate::core::probe::query;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use wasmi::{
    Caller, Engine, Func, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
};

static ENGINE: LazyLock<Engine> = LazyLock::new(|| {
    let mut config = wasmi::Config::default();
    config.consume_fuel(true);
    Engine::new(&config)
});
static MODULES: LazyLock<parking_lot::Mutex<HashMap<String, Arc<Module>>>> =
    LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));
const FINGERPRINT: &[u8] = b"unknown|unknown|unknown|unknown";
struct File {
    bytes: Vec<u8>,
    position: usize,
    path: String,
    flags: i32,
}
struct Host {
    limits: StoreLimits,
    public: [u8; 32],
    signature: [u8; 64],
    files: BTreeMap<String, Vec<u8>>,
    open: HashMap<i32, Arc<parking_lot::Mutex<File>>>,
    modern: bool,
    domain_allowed: bool,
    cancel: CancellationToken,
    expires: Instant,
    io_bytes: usize,
}
impl Host {
    fn new(modern: bool, origin: &str, cancel: CancellationToken) -> Self {
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        let key = SigningKey::from_bytes(&seed);
        let public = key.verifying_key().to_bytes();
        let signature = key.sign(FINGERPRINT).to_bytes();
        let mut files = BTreeMap::new();
        let prefix = "/Beacon_1A2B3C4D5E6F7G8H_9Z";
        files.insert(format!("{prefix}/f1.dat"), public.to_vec());
        files.insert(format!("{prefix}/f2.dat"), signature.to_vec());
        let mut f3 = vec![0; 64];
        rand::rngs::OsRng.fill_bytes(&mut f3);
        for (index, byte) in f3.iter_mut().enumerate() {
            *byte ^= (3 * index + 17) as u8;
        }
        files.insert(format!("{prefix}/f3.dat"), f3);
        let mut f4 = vec![0; 32];
        rand::rngs::OsRng.fill_bytes(&mut f4);
        for (index, byte) in f4.iter_mut().enumerate() {
            *byte = public[index].wrapping_add(*byte & 127);
        }
        files.insert(format!("{prefix}/f4.dat"), f4);
        let domain_allowed = url::Url::parse(origin)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .is_some_and(|host| {
                ["miguvideo.com", "cmcc-vr.com"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            });
        Self {
            limits: StoreLimitsBuilder::new()
                .memory_size(64 * 1024 * 1024)
                .build(),
            public,
            signature,
            files,
            open: HashMap::new(),
            modern,
            domain_allowed,
            cancel,
            expires: Instant::now() + Duration::from_secs(5),
            io_bytes: 0,
        }
    }
}
fn error(message: &str) -> wasmi::Error {
    wasmi::Error::new(message.to_owned())
}
fn check(host: &Host) -> Result<(), wasmi::Error> {
    if host.cancel.is_cancelled() {
        return Err(error("探测已取消"));
    }
    if Instant::now() >= host.expires {
        return Err(error("咪咕签名计算超时"));
    }
    Ok(())
}
// fuel 不计算宿主 I/O 成本；所有调用共享总字节预算。
fn charge(caller: &mut Caller<'_, Host>, length: usize) -> Result<(), wasmi::Error> {
    check(caller.data())?;
    caller.data_mut().io_bytes = caller
        .data()
        .io_bytes
        .checked_add(length)
        .filter(|n| *n <= 16 * 1024 * 1024)
        .ok_or_else(|| error("咪咕 WASM I/O 过大"))?;
    Ok(())
}
fn memory(caller: &Caller<'_, Host>) -> Result<Memory, wasmi::Error> {
    check(caller.data())?;
    caller
        .get_export(if caller.data().modern { "m" } else { "d" })
        .and_then(wasmi::Extern::into_memory)
        .ok_or_else(|| error("咪咕 WASM 内存导出缺失"))
}
fn read_i32(caller: &Caller<'_, Host>, address: usize) -> Result<i32, wasmi::Error> {
    let mut bytes = [0; 4];
    memory(caller)?
        .read(caller, address, &mut bytes)
        .map_err(|_| error("咪咕 WASM 内存访问越界"))?;
    Ok(i32::from_le_bytes(bytes))
}
fn write(caller: &mut Caller<'_, Host>, address: usize, bytes: &[u8]) -> Result<(), wasmi::Error> {
    memory(caller)?
        .write(caller, address, bytes)
        .map_err(|_| error("咪咕 WASM 内存访问越界"))
}
fn c_string(caller: &Caller<'_, Host>, address: i32) -> Result<String, wasmi::Error> {
    let memory = memory(caller)?;
    let bytes = memory.data(caller);
    let tail = bytes
        .get(address as usize..)
        .ok_or_else(|| error("咪咕 WASM 字符串指针无效"))?;
    let end = tail
        .iter()
        .take(1024 * 1024)
        .position(|b| *b == 0)
        .ok_or_else(|| error("咪咕 WASM 字符串未终止"))?;
    String::from_utf8(tail[..end].to_vec()).map_err(|_| error("咪咕 WASM 字符串编码无效"))
}
fn io_write(
    mut caller: Caller<'_, Host>,
    fd: i32,
    iov: i32,
    count: i32,
    out: i32,
) -> Result<i32, wasmi::Error> {
    if !(0..=4096).contains(&count) {
        return Err(error("咪咕 WASM iovec 数量无效"));
    }
    let mut total = 0usize;
    for index in 0..count as usize {
        let ptr = read_i32(&caller, iov as usize + index * 8)? as usize;
        let length = read_i32(&caller, iov as usize + index * 8 + 4)? as usize;
        charge(&mut caller, length)?;
        if fd <= 2 {
            let data = memory(&caller)?.data(&caller);
            data.get(
                ptr..ptr
                    .checked_add(length)
                    .ok_or_else(|| error("咪咕 WASM 地址溢出"))?,
            )
            .ok_or_else(|| error("咪咕 WASM I/O 越界"))?;
        } else {
            let mut bytes = vec![0; length];
            memory(&caller)?
                .read(&caller, ptr, &mut bytes)
                .map_err(|_| error("咪咕 WASM I/O 越界"))?;
            let Some(file) = caller.data().open.get(&fd).cloned() else {
                return Ok(8);
            };
            let mut file = file.lock();
            let end = file
                .position
                .checked_add(length)
                .filter(|n| *n <= 16 * 1024 * 1024)
                .ok_or_else(|| error("咪咕 WASM 文件过大"))?;
            let position = file.position;
            if end > file.bytes.len() {
                file.bytes.resize(end, 0);
            }
            file.bytes[position..end].copy_from_slice(&bytes);
            file.position = end;
        }
        total += length;
    }
    write(&mut caller, out as usize, &(total as u32).to_le_bytes())?;
    Ok(0)
}
fn io_read(
    mut caller: Caller<'_, Host>,
    fd: i32,
    iov: i32,
    count: i32,
    out: i32,
) -> Result<i32, wasmi::Error> {
    if !(0..=4096).contains(&count) {
        return Err(error("咪咕 WASM iovec 数量无效"));
    }
    let mut total = 0usize;
    for index in 0..count as usize {
        let ptr = read_i32(&caller, iov as usize + index * 8)? as usize;
        let length = read_i32(&caller, iov as usize + index * 8 + 4)? as usize;
        charge(&mut caller, length)?;
        let bytes = if fd == 0 {
            Vec::new()
        } else {
            let Some(file) = caller.data().open.get(&fd).cloned() else {
                return Ok(8);
            };
            let mut file = file.lock();
            if matches!(file.path.as_str(), "/dev/urandom" | "/dev/random") {
                let mut bytes = vec![0; length];
                rand::rngs::OsRng.fill_bytes(&mut bytes);
                bytes
            } else {
                let end = file.position.saturating_add(length).min(file.bytes.len());
                let bytes = file.bytes.get(file.position..end).unwrap_or(&[]).to_vec();
                file.position = end;
                bytes
            }
        };
        write(&mut caller, ptr, &bytes)?;
        total += bytes.len();
        if bytes.len() < length {
            break;
        }
    }
    write(&mut caller, out as usize, &(total as u32).to_le_bytes())?;
    Ok(0)
}
fn em_asm(
    mut caller: Caller<'_, Host>,
    code: i32,
    _signature: i32,
    arguments: i32,
) -> Result<i32, wasmi::Error> {
    match code {
        14363 => {
            let public_ptr = read_i32(&caller, arguments as usize)? as usize;
            let signature_ptr = read_i32(&caller, arguments as usize + 4)? as usize;
            let public = caller.data().public;
            let signature = caller.data().signature;
            write(&mut caller, public_ptr, &public)?;
            write(&mut caller, signature_ptr, &signature)?;
            Ok(1)
        }
        15285 => {
            let result_ptr = read_i32(&caller, arguments as usize)? as usize;
            let verify = caller
                .get_export("o")
                .and_then(wasmi::Extern::into_func)
                .ok_or_else(|| error("咪咕 WASM 验证函数缺失"))?;
            let result = verify.typed::<(), i32>(&caller)?.call(&mut caller, ())?;
            write(&mut caller, result_ptr, &result.to_le_bytes())?;
            let callback = caller
                .get_export("s")
                .and_then(wasmi::Extern::into_func)
                .ok_or_else(|| error("咪咕 WASM 验证回调缺失"))?;
            callback
                .typed::<i32, ()>(&caller)?
                .call(&mut caller, result)?;
            Ok(0)
        }
        _ => Err(error("咪咕 WASM 请求了未知宿主回调")),
    }
}
fn modern_imports(linker: &mut Linker<Host>) -> Result<(), wasmi::Error> {
    linker.func_wrap("a", "a", |mut c: Caller<'_, Host>, fd: i32| {
        if fd <= 2 {
            return 0;
        }
        if let Some(file) = c.data_mut().open.remove(&fd) {
            if let Ok(file) = Arc::try_unwrap(file) {
                let file = file.into_inner();
                c.data_mut().files.insert(file.path, file.bytes);
            }
            0
        } else {
            8
        }
    })?;
    linker.func_wrap("a", "b", io_write)?;
    linker.func_wrap("a", "f", io_read)?;
    linker.func_wrap("a", "d", em_asm)?;
    linker.func_wrap(
        "a",
        "c",
        |mut c: Caller<'_, Host>, fd: i32, command: i32, arg: i32| -> Result<i32, wasmi::Error> {
            if fd > 2 && !c.data().open.contains_key(&fd) {
                return Ok(-8);
            }
            match command {
                1 | 2 | 13 | 14 => Ok(0),
                3 => Ok(c.data().open.get(&fd).map_or(0, |f| f.lock().flags)),
                4 => {
                    let flags = read_i32(&c, arg as usize)?;
                    if let Some(file) = c.data().open.get(&fd) {
                        file.lock().flags |= flags;
                    }
                    Ok(0)
                }
                0 => {
                    let minimum = read_i32(&c, arg as usize)?;
                    let descriptor = (minimum.max(3)..4096)
                        .find(|d| !c.data().open.contains_key(d))
                        .ok_or_else(|| error("咪咕 WASM 文件描述符耗尽"))?;
                    let Some(file) = c.data().open.get(&fd).cloned() else {
                        return Ok(-8);
                    };
                    c.data_mut().open.insert(descriptor, file);
                    Ok(descriptor)
                }
                12 => {
                    write(&mut c, arg as usize, &2i16.to_le_bytes())?;
                    Ok(0)
                }
                _ => Ok(-28),
            }
        },
    )?;
    linker.func_wrap(
        "a",
        "g",
        |c: Caller<'_, Host>, fd: i32, command: i32, _arg: i32| {
            if fd > 2 && !c.data().open.contains_key(&fd) {
                -8
            } else {
                match command {
                    21509 | 21510 | 21511 | 21512 | 21524 | 21515 | 21505 | 21506 | 21507
                    | 21508 | 21519 | 21523 => {
                        if fd <= 2 {
                            0
                        } else {
                            -59
                        }
                    }
                    21520 => -28,
                    _ => -28,
                }
            }
        },
    )?;
    linker.func_wrap(
        "a",
        "h",
        |mut c: Caller<'_, Host>,
         directory: i32,
         path: i32,
         flags: i32,
         _mode: i32|
         -> Result<i32, wasmi::Error> {
            let mut path = c_string(&c, path)?;
            if !path.starts_with('/') {
                if directory != -100 {
                    return Ok(-8);
                }
                path = format!("/{path}");
            }
            if path.split('/').any(|part| part == "..") {
                return Ok(-2);
            }
            let exists = c.data().files.contains_key(&path)
                || matches!(path.as_str(), "/dev/urandom" | "/dev/random" | "/dev/null");
            if !exists && flags & 64 == 0 {
                return Ok(-44);
            }
            let bytes = if flags & 512 != 0 {
                vec![]
            } else {
                c.data().files.get(&path).cloned().unwrap_or_default()
            };
            let descriptor = (3..4096)
                .find(|d| !c.data().open.contains_key(d))
                .ok_or_else(|| error("咪咕 WASM 文件描述符耗尽"))?;
            c.data_mut().open.insert(
                descriptor,
                Arc::new(parking_lot::Mutex::new(File {
                    bytes,
                    position: 0,
                    path,
                    flags,
                })),
            );
            Ok(descriptor)
        },
    )?;
    linker.func_wrap(
        "a",
        "e",
        |mut c: Caller<'_, Host>| -> Result<i32, wasmi::Error> {
            let malloc = c
                .get_export("p")
                .and_then(wasmi::Extern::into_func)
                .ok_or_else(|| error("咪咕 WASM malloc 缺失"))?;
            let ptr = malloc
                .typed::<i32, i32>(&c)?
                .call(&mut c, (FINGERPRINT.len() + 1) as i32)?;
            write(&mut c, ptr as usize, FINGERPRINT)?;
            write(&mut c, ptr as usize + FINGERPRINT.len(), &[0])?;
            Ok(ptr)
        },
    )?;
    // 签名上下文来自咪咕平台 URL，与 SDK 的合法域名检查一致。
    linker.func_wrap("a", "i", |caller: Caller<'_, Host>| {
        i32::from(caller.data().domain_allowed)
    })?;
    linker.func_wrap("a", "j", |_size: i32| -> Result<i32, wasmi::Error> {
        Err(error("咪咕 WASM 内存增长超出资源限制"))
    })?;
    linker.func_wrap(
        "a",
        "k",
        |mut c: Caller<'_, Host>,
         fd: i32,
         offset: i64,
         origin: i32,
         out: i32|
         -> Result<i32, wasmi::Error> {
            let Some(file) = c.data().open.get(&fd).cloned() else {
                return Ok(8);
            };
            let mut file = file.lock();
            let base = match origin {
                0 => 0,
                1 => file.position as i64,
                2 => file.bytes.len() as i64,
                _ => return Ok(28),
            };
            let Some(position) = base
                .checked_add(offset)
                .filter(|p| *p >= 0 && *p <= 16 * 1024 * 1024)
            else {
                return Ok(28);
            };
            file.position = position as usize;
            write(&mut c, out as usize, &position.to_le_bytes())?;
            Ok(0)
        },
    )?;
    linker.func_wrap(
        "a",
        "l",
        |_a: i32, _b: i32, _c: i32, _d: i32| -> Result<(), wasmi::Error> {
            Err(error("咪咕 WASM 断言失败"))
        },
    )?;
    Ok(())
}
fn allocate(
    store: &mut Store<Host>,
    instance: &Instance,
    memory: &Memory,
    name: &str,
    value: &str,
) -> Result<i32, String> {
    let length = value
        .len()
        .checked_add(1)
        .filter(|n| *n <= 1024 * 1024)
        .ok_or("咪咕签名参数过大")?;
    let ptr = instance
        .get_typed_func::<i32, i32>(&*store, name)
        .map_err(|e| e.to_string())?
        .call(&mut *store, length as i32)
        .map_err(|e| e.to_string())?;
    memory
        .write(&mut *store, ptr as usize, value.as_bytes())
        .map_err(|_| "咪咕 WASM 内存访问越界".to_string())?;
    memory
        .write(&mut *store, ptr as usize + value.len(), &[0])
        .map_err(|_| "咪咕 WASM 内存访问越界".to_string())?;
    Ok(ptr)
}
fn string(
    store: &Store<Host>,
    memory: &Memory,
    ptr: i32,
    maximum: usize,
) -> Result<String, String> {
    let data = memory.data(store);
    let bytes = data.get(ptr as usize..).ok_or("咪咕签名输出指针无效")?;
    let end = bytes
        .iter()
        .take(maximum)
        .position(|v| *v == 0)
        .ok_or("咪咕签名输出未终止")?;
    String::from_utf8(bytes[..end].to_vec()).map_err(|_| "咪咕签名输出编码无效".into())
}
pub(super) fn calculate(
    version: &str,
    bytes: &[u8],
    url: &str,
    factor: &str,
    origin: &str,
    cancel: CancellationToken,
) -> Result<String, String> {
    let module = {
        let mut cache = MODULES.lock();
        if let Some(module) = cache.get(version) {
            module.clone()
        } else {
            let module =
                Arc::new(Module::new(&ENGINE, bytes).map_err(|e| format!("咪咕 WASM 无效：{e}"))?);
            if cache.len() >= 4 {
                cache.clear();
            }
            cache.insert(version.into(), module.clone());
            module
        }
    };
    let modern = module.exports().any(|export| export.name() == "m");
    let names = if modern {
        [
            "m", "p", "y", "t", "u", "v", "w", "z", "A", "B", "C", "D", "F",
        ]
    } else {
        [
            "d", "u", "m", "h", "i", "j", "k", "n", "o", "p", "q", "r", "t",
        ]
    };
    let mut store = Store::new(&ENGINE, Host::new(modern, origin, cancel));
    store.limiter(|host| &mut host.limits);
    store.set_fuel(10_000_000).map_err(|e| e.to_string())?;
    let mut linker = Linker::new(&ENGINE);
    if modern {
        modern_imports(&mut linker).map_err(|e| e.to_string())?;
    } else {
        linker
            .func_wrap("a", "a", io_write)
            .map_err(|e| e.to_string())?;
        // 固定版 migu.js 的 b/c 没有副作用，返回值按 JS undefined 的数值转换处理。
        for import in module
            .imports()
            .filter(|i| i.module() == "a" && matches!(i.name(), "b" | "c"))
        {
            let ty = import.ty().func().ok_or("咪咕 WASM ABI 不兼容")?.clone();
            let function = Func::new(&mut store, ty, |_, _, results| {
                for result in results {
                    *result = wasmi::Val::default_for_ty(result.ty());
                }
                Ok(())
            });
            linker
                .define("a", import.name(), function)
                .map_err(|e| e.to_string())?;
        }
    }
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| format!("咪咕 WASM ABI 不兼容：{e}"))?;
    if modern {
        instance
            .get_typed_func::<(), ()>(&store, "n")
            .map_err(|e| e.to_string())?
            .call(&mut store, ())
            .map_err(|e| e.to_string())?;
    }
    let memory = instance
        .get_memory(&store, names[0])
        .ok_or("咪咕 WASM 内存导出缺失")?;
    let values = [
        query(url, "userid").unwrap_or_default(),
        query(url, "timestamp").unwrap_or_default(),
        query(url, "ProgramID").unwrap_or_default(),
        query(url, "Channel_ID").unwrap_or_default(),
        query(url, "puData").unwrap_or_default(),
        factor.to_owned(),
    ];
    let mut pointers = [0i32; 6];
    for (i, value) in values.iter().enumerate() {
        pointers[i] = allocate(&mut store, &instance, &memory, names[1], value)?;
    }
    let output = allocate(&mut store, &instance, &memory, names[1], &" ".repeat(127))?;
    let intermediate = allocate(&mut store, &instance, &memory, names[1], &" ".repeat(127))?;
    let context = instance
        .get_typed_func::<(), i32>(&store, names[2])
        .map_err(|e| e.to_string())?
        .call(&mut store, ())
        .map_err(|e| e.to_string())?;
    for (name, index) in [
        (names[3], Some(2)),
        (names[10], Some(1)),
        (names[9], Some(0)),
        (names[5], None),
        (names[11], None),
        (names[8], Some(4)),
        (names[4], Some(3)),
    ] {
        check(store.data()).map_err(|e| e.to_string())?;
        let (pointer, length) = index
            .map(|index| (pointers[index], values[index].len() as i32))
            .unwrap_or((0, 0));
        let result = instance
            .get_typed_func::<(i32, i32, i32), i32>(&store, name)
            .map_err(|e| e.to_string())?
            .call(&mut store, (context, pointer, length))
            .map_err(|e| e.to_string())?;
        if result == -1 {
            return Err("咪咕 WASM 参数校验失败".into());
        }
    }
    let result = instance
        .get_typed_func::<(i32, i32, i32, i32, i32), i32>(&store, names[12])
        .map_err(|e| e.to_string())?
        .call(
            &mut store,
            (
                context,
                pointers[5],
                values[5].len() as i32,
                intermediate,
                128,
            ),
        )
        .map_err(|e| e.to_string())?;
    if result == -1 {
        return Err("咪咕 WASM 加密因子无效".into());
    }
    let intermediate_value = string(&store, &memory, intermediate, 128)?;
    let intermediate_ptr = allocate(
        &mut store,
        &instance,
        &memory,
        names[1],
        &intermediate_value,
    )?;
    let result = instance
        .get_typed_func::<(i32, i32, i32), i32>(&store, names[7])
        .map_err(|e| e.to_string())?
        .call(
            &mut store,
            (context, intermediate_ptr, intermediate_value.len() as i32),
        )
        .map_err(|e| e.to_string())?;
    if result == -1 {
        return Err("咪咕 WASM 中间签名无效".into());
    }
    let result = instance
        .get_typed_func::<(i32, i32, i32), i32>(&store, names[6])
        .map_err(|e| e.to_string())?
        .call(&mut store, (context, output, 128))
        .map_err(|e| e.to_string())?;
    if result != 0 {
        return Err(format!("咪咕 WASM 签名失败：{result}"));
    }
    let value = string(&store, &memory, output, 128)?;
    if value.is_empty() || value.starts_with('-') {
        return Err("咪咕 WASM 未返回有效签名".into());
    }
    Ok(value)
}
pub(super) fn is_modern(bytes: &[u8]) -> Result<bool, String> {
    if bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err("咪咕 WASM 文件头无效".into());
    }
    fn leb(bytes: &[u8], position: &mut usize) -> Result<usize, String> {
        let mut value = 0usize;
        for index in 0..5 {
            let byte = *bytes.get(*position).ok_or("咪咕 WASM 数据截断")?;
            *position += 1;
            if index == 4 && byte > 15 {
                return Err("咪咕 WASM 长度溢出".into());
            }
            value |= ((byte & 127) as usize) << (index * 7);
            if byte < 128 {
                return Ok(value);
            }
        }
        Err("咪咕 WASM 长度无效".into())
    }
    let mut position = 8;
    while position < bytes.len() {
        let kind = bytes[position];
        position += 1;
        let length = leb(bytes, &mut position)?;
        let end = position
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or("咪咕 WASM 数据截断")?;
        if kind == 7 {
            let section = &bytes[position..end];
            let mut offset = 0;
            let count = leb(section, &mut offset)?;
            for _ in 0..count {
                let length = leb(section, &mut offset)?;
                let name = section
                    .get(offset..offset + length)
                    .ok_or("咪咕 WASM 导出截断")?;
                offset += length;
                let kind = *section.get(offset).ok_or("咪咕 WASM 导出截断")?;
                offset += 1;
                let _index = leb(section, &mut offset)?;
                if name == b"m" && kind == 2 {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        position = end;
    }
    Err("咪咕 WASM 导出缺失".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    #[test]
    fn real_sdk_obeys_fuel_limit() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/migu-wasm.json")).unwrap();
        let bytes = STANDARD
            .decode(fixture["bytes_base64"].as_str().unwrap())
            .unwrap();
        assert!(is_modern(&bytes).unwrap());
        let result=calculate(fixture["version"].as_str().unwrap(),&bytes,"https://media.example/live.flv?userid=fixture&timestamp=1760000000123&ProgramID=10001&Channel_ID=H5&puData=invalid","BjfS7eNf3OIROs2T1E8hHQ==","https://www.miguvideo.com",CancellationToken::new());
        assert_eq!(result.unwrap_err(), "all fuel consumed by WebAssembly");
    }
}
