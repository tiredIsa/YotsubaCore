use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::MacosLauncher;
use url::Url;
use percent_encoding::percent_decode_str;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const PROFILE_FILE: &str = "profile.json";
const PROFILE_STATE_FILE: &str = "profile.state.json";
const APP_STATE_FILE: &str = "app.state.json";
const CONFIG_FILE: &str = "singbox.generated.json";
const LOG_FILE: &str = "singbox.log";
const BIN_DIR: &str = "bin";
const RULE_SET_DIR: &str = "rule-sets";
const SINGBOX_EXE: &str = "sing-box.exe";
const LOG_MAX_BYTES: u64 = 8 * 1024 * 1024;
const LOG_KEEP_BYTES: u64 = 6 * 1024 * 1024;
const LOCAL_PROXY_HOST: &str = "127.0.0.1";
const LOCAL_PROXY_PORT: u16 = 2080;
const LOCAL_PROXY_TAG: &str = "local-proxy";
const AUTOSTART_ARG: &str = "--autostart";
const TRAY_OPEN_ID: &str = "tray-open";
const TRAY_EXIT_ID: &str = "tray-exit";
const GEOIP_RU_TAG: &str = "geoip-ru";
const GEOIP_RU_FILE: &str = "geoip-ru.srs";
const GEOIP_RU_URL: &str =
    "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/geoip-ru.srs";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ProxyMode {
    Off,
    Selected,
    Full,
}

impl Default for ProxyMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum AppRuleMode {
    Proxy,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppRule {
    path: String,
    mode: AppRuleMode,
    name: Option<String>,
}

#[derive(Default)]
struct ProxyState {
    child: Option<Child>,
    mode: ProxyMode,
    last_exit: Option<i32>,
    last_error: Option<String>,
    config_path: Option<PathBuf>,
    watch_token: u64,
    #[cfg(target_os = "windows")]
    job: Option<JobHandle>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProxyStatus {
    running: bool,
    mode: ProxyMode,
    pid: Option<u32>,
    last_exit: Option<i32>,
    last_error: Option<String>,
    config_path: Option<String>,
    profile_path: String,
    log_path: Option<String>,
}

#[derive(Serialize, Clone)]
struct ProxyExitPayload {
    code: Option<i32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LogBatchPayload {
    lines: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProcessEntry {
    name: String,
    path: String,
    count: usize,
    pids: Vec<u32>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileState {
    active_tag: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AppState {
    last_mode: ProxyMode,
    app_rules: Vec<AppRule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileData {
    outbounds: Vec<Value>,
    active_tag: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    profile: ProfileData,
    added: usize,
    errors: Vec<String>,
}

type SharedState = Arc<Mutex<ProxyState>>;

#[derive(Default)]
struct ExitFlag(AtomicBool);

impl ExitFlag {
    fn allow_exit(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn is_allowed(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

struct TrayState {
    _tray: tauri::tray::TrayIcon,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct JobHandle(isize);

#[cfg(target_os = "windows")]
impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
fn create_job_object() -> Result<JobHandle, String> {
    let handle = unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
    if handle == 0 {
        return Err(err(
            "JOB_ERROR",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let result = unsafe {
        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if result == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(err(
            "JOB_ERROR",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    Ok(JobHandle(handle))
}

fn err(tag: &str, detail: impl AsRef<str>) -> String {
    format!("{tag}|{}", detail.as_ref())
}

fn ensure_app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| err("PATH_ERROR", e.to_string()))?;
    fs::create_dir_all(&dir).map_err(|e| err("PATH_ERROR", e.to_string()))?;
    Ok(dir)
}

fn resolve_profile_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_app_data_dir(app)?.join(PROFILE_FILE))
}

fn resolve_profile_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_app_data_dir(app)?.join(PROFILE_STATE_FILE))
}

fn resolve_app_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_app_data_dir(app)?.join(APP_STATE_FILE))
}

fn resolve_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_app_data_dir(app)?.join(CONFIG_FILE))
}

fn resolve_log_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_app_data_dir(app)?.join(LOG_FILE))
}

fn resolve_rule_set_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = ensure_app_data_dir(app)?.join(RULE_SET_DIR);
    fs::create_dir_all(&dir).map_err(|e| err("PATH_ERROR", e.to_string()))?;
    Ok(dir)
}

fn resolve_rule_set_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(resolve_rule_set_dir(app)?.join(name))
}

fn default_profile() -> Value {
    json!({
        "outbounds": [
            {
                "type": "socks",
                "tag": "proxy",
                "server": "example.com",
                "server_port": 1080
            },
            {
                "type": "direct",
                "tag": "direct"
            }
        ]
    })
}

fn ensure_profile(app: &AppHandle) -> Result<(Value, PathBuf), String> {
    let profile_path = resolve_profile_path(app)?;
    if !profile_path.exists() {
        let content = serde_json::to_string_pretty(&default_profile())
            .map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
        fs::write(&profile_path, content)
            .map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
        return Err(err("PROFILE_MISSING", profile_path.display().to_string()));
    }

    let raw = fs::read_to_string(&profile_path)
        .map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    Ok((value, profile_path))
}

fn load_profile_json(app: &AppHandle) -> Result<Value, String> {
    match ensure_profile(app) {
        Ok((value, _)) => Ok(value),
        Err(message) if message.starts_with("PROFILE_MISSING|") => {
            let profile_path = resolve_profile_path(app)?;
            let raw = fs::read_to_string(&profile_path)
                .map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
            let value: Value =
                serde_json::from_str(&raw).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
            Ok(value)
        }
        Err(message) => Err(message),
    }
}

fn save_profile_json(app: &AppHandle, profile: &Value) -> Result<(), String> {
    let profile_path = resolve_profile_path(app)?;
    let content =
        serde_json::to_string_pretty(profile).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    fs::write(&profile_path, content).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    Ok(())
}

fn load_profile_state(app: &AppHandle) -> ProfileState {
    let path = match resolve_profile_state_path(app) {
        Ok(path) => path,
        Err(_) => return ProfileState::default(),
    };
    if !path.exists() {
        return ProfileState::default();
    }
    let raw = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(_) => return ProfileState::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_profile_state(app: &AppHandle, state: &ProfileState) -> Result<(), String> {
    let path = resolve_profile_state_path(app)?;
    let content =
        serde_json::to_string_pretty(state).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    fs::write(&path, content).map_err(|e| err("PROFILE_INVALID", e.to_string()))?;
    Ok(())
}

fn load_app_state(app: &AppHandle) -> AppState {
    let path = match resolve_app_state_path(app) {
        Ok(path) => path,
        Err(_) => return AppState::default(),
    };
    if !path.exists() {
        return AppState::default();
    }
    let raw = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(_) => return AppState::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_app_state(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let path = resolve_app_state_path(app)?;
    let content =
        serde_json::to_string_pretty(state).map_err(|e| err("STATE_INVALID", e.to_string()))?;
    fs::write(&path, content).map_err(|e| err("STATE_INVALID", e.to_string()))?;
    Ok(())
}

fn profile_data(app: &AppHandle, profile: &Value) -> ProfileData {
    let outbounds = profile
        .get("outbounds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let state = load_profile_state(app);
    ProfileData {
        outbounds,
        active_tag: state.active_tag,
    }
}

fn ensure_singbox_exe(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|e| err("PATH_ERROR", e.to_string()))?;

    let candidates = [
        resource_dir.join(SINGBOX_EXE),
        resource_dir.join("resources").join(SINGBOX_EXE),
    ];

    let resource_path = candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .ok_or_else(|| err("SINGBOX_MISSING", candidates[0].display().to_string()))?;

    let bin_dir = ensure_app_data_dir(app)?.join(BIN_DIR);
    fs::create_dir_all(&bin_dir).map_err(|e| err("PATH_ERROR", e.to_string()))?;
    let target_path = bin_dir.join(SINGBOX_EXE);

    let copy_needed = match (fs::metadata(&resource_path), fs::metadata(&target_path)) {
        (Ok(src_meta), Ok(dst_meta)) => src_meta.len() != dst_meta.len(),
        _ => true,
    };

    if copy_needed {
        fs::copy(&resource_path, &target_path)
            .map_err(|e| err("SINGBOX_MISSING", e.to_string()))?;
    }

    Ok(target_path)
}

fn is_process_name(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"');
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('\\') || trimmed.contains('/') {
        return false;
    }
    if trimmed.contains(':') {
        return false;
    }
    true
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn normalize_rules(rules: Vec<AppRule>) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut proxy_paths: Vec<String> = Vec::new();
    let mut direct_paths: Vec<String> = Vec::new();
    let mut proxy_names: Vec<String> = Vec::new();
    let mut direct_names: Vec<String> = Vec::new();
    for rule in rules {
        let path = rule.path.trim().trim_matches('"').to_string();
        if path.is_empty() {
            continue;
        }
        let is_name = is_process_name(&path);
        match rule.mode {
            AppRuleMode::Proxy => {
                if is_name {
                    proxy_names.push(path);
                } else {
                    proxy_paths.push(path);
                }
            }
            AppRuleMode::Direct => {
                if is_name {
                    direct_names.push(path);
                } else {
                    direct_paths.push(path);
                }
            }
        }
    }
    sort_dedup(&mut proxy_paths);
    sort_dedup(&mut direct_paths);
    sort_dedup(&mut proxy_names);
    sort_dedup(&mut direct_names);
    (proxy_paths, direct_paths, proxy_names, direct_names)
}

fn push_process_rules(
    rules: &mut Vec<Value>,
    paths: &[String],
    names: &[String],
    outbound: &str,
) {
    if !paths.is_empty() {
        rules.push(json!({
            "process_path": paths,
            "outbound": outbound
        }));
    }
    if !names.is_empty() {
        rules.push(json!({
            "process_name": names,
            "outbound": outbound
        }));
    }
}

fn build_geoip_ru_rule_set(app: &AppHandle) -> Result<Value, String> {
    let path = resolve_rule_set_path(app, GEOIP_RU_FILE)?;
    if path.exists() {
        Ok(json!({
            "tag": GEOIP_RU_TAG,
            "type": "local",
            "format": "binary",
            "path": path.display().to_string()
        }))
    } else {
        Ok(json!({
            "tag": GEOIP_RU_TAG,
            "type": "remote",
            "format": "binary",
            "url": GEOIP_RU_URL,
            "download_detour": "proxy",
            "update_interval": "72h"
        }))
    }
}

fn push_ru_bypass_rules(rules: &mut Vec<Value>) {
    rules.push(json!({
        "domain_suffix": [".ru"],
        "outbound": "direct"
    }));
    rules.push(json!({
        "rule_set": [GEOIP_RU_TAG],
        "outbound": "direct"
    }));
}

fn build_config(app: &AppHandle, mode: ProxyMode, rules: Vec<AppRule>) -> Result<PathBuf, String> {
    let (mut profile, _profile_path) = ensure_profile(app)?;
    let log_path = resolve_log_path(app)?;

    let profile_obj = profile
        .as_object_mut()
        .ok_or_else(|| err("PROFILE_INVALID", "root must be an object"))?;

    let outbounds_value = profile_obj
        .get("outbounds")
        .cloned()
        .ok_or_else(|| err("PROFILE_OUTBOUNDS_MISSING", "missing outbounds"))?;

    let mut outbounds = outbounds_value
        .as_array()
        .cloned()
        .ok_or_else(|| err("PROFILE_INVALID", "outbounds must be an array"))?;

    let mut tags: Vec<String> = outbounds
        .iter()
        .filter_map(|item| item.get("tag").and_then(Value::as_str))
        .map(|tag| tag.to_string())
        .collect();

    if tags.is_empty() {
        return Err(err("PROFILE_OUTBOUNDS_MISSING", "no outbounds"));
    }

    let mut used_tags: HashSet<String> = tags.iter().cloned().collect();
    let proxy_index = outbounds.iter().position(|item| {
        item.get("tag")
            .and_then(Value::as_str)
            .map(|tag| tag == "proxy")
            .unwrap_or(false)
    });
    let state = load_profile_state(app);
    let mut active_tag = state.active_tag;
    if let Some(tag) = active_tag.clone() {
        if !tags.contains(&tag) {
            active_tag = None;
        }
    }

    if let Some(index) = proxy_index {
        let proxy_type = outbounds[index]
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let needs_selector = active_tag.is_some() && tags.len() > 1;
        if proxy_type == "selector" {
            let selector_tags: Vec<String> = tags
                .iter()
                .filter(|tag| *tag != "proxy" && *tag != "direct")
                .cloned()
                .collect();
            if !selector_tags.is_empty() {
                outbounds[index]["outbounds"] = json!(selector_tags);
            }
            if let Some(tag) = active_tag {
                outbounds[index]["default"] = json!(tag);
            }
        } else if needs_selector {
            let renamed = unique_tag("proxy-origin", &mut used_tags);
            outbounds[index]["tag"] = json!(renamed.clone());
            tags = outbounds
                .iter()
                .filter_map(|item| item.get("tag").and_then(Value::as_str))
                .map(|tag| tag.to_string())
                .collect();
        let selected_tag = active_tag.unwrap_or_else(|| renamed.clone());
        let selector_tags: Vec<String> = tags
            .iter()
            .filter(|tag| *tag != "proxy" && *tag != "direct")
            .cloned()
            .collect();
        if selector_tags.is_empty() {
            return Err(err("PROFILE_OUTBOUNDS_MISSING", "no proxy outbounds"));
        }
        outbounds.push(json!({
            "type": "selector",
            "tag": "proxy",
            "outbounds": selector_tags,
            "default": selected_tag
        }));
            tags = outbounds
                .iter()
                .filter_map(|item| item.get("tag").and_then(Value::as_str))
                .map(|tag| tag.to_string())
                .collect();
        }
    } else {
        let selected_tag = active_tag.unwrap_or_else(|| tags[0].clone());
        let selector_tags: Vec<String> = tags
            .iter()
            .filter(|tag| *tag != "proxy" && *tag != "direct")
            .cloned()
            .collect();
        if selector_tags.is_empty() {
            return Err(err("PROFILE_OUTBOUNDS_MISSING", "no proxy outbounds"));
        }
        outbounds.push(json!({
            "type": "selector",
            "tag": "proxy",
            "outbounds": selector_tags,
            "default": selected_tag
        }));
        tags = outbounds
            .iter()
            .filter_map(|item| item.get("tag").and_then(Value::as_str))
            .map(|tag| tag.to_string())
            .collect();
    }

    let has_direct = tags.iter().any(|tag| tag == "direct");
    if !has_direct {
        outbounds.push(json!({
            "type": "direct",
            "tag": "direct"
        }));
    }

    profile_obj.insert("outbounds".to_string(), Value::Array(outbounds));

    if !profile_obj.contains_key("log") {
        profile_obj.insert(
            "log".to_string(),
            json!( {
                "level": "info",
                "output": log_path
            }),
        );
    }

    if !profile_obj.contains_key("dns") {
        profile_obj.insert(
            "dns".to_string(),
            json!({
                "servers": [
                    {
                        "tag": "dns-local",
                        "type": "local"
                    },
                    {
                        "tag": "dns-remote",
                        "type": "https",
                        "server": "dns.google",
                        "path": "/dns-query",
                        "domain_resolver": "dns-local"
                    }
                ],
                "final": "dns-remote"
            }),
        );
    }

    let mut inbounds = vec![json!({
        "type": "tun",
        "tag": "tun-in",
        "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
        "auto_route": true,
        "strict_route": true,
        "stack": "system"
    })];
    inbounds.push(json!({
        "type": "mixed",
        "tag": LOCAL_PROXY_TAG,
        "listen": LOCAL_PROXY_HOST,
        "listen_port": LOCAL_PROXY_PORT
    }));
    profile_obj.insert("inbounds".to_string(), Value::Array(inbounds));

    let geoip_ru_rule_set = build_geoip_ru_rule_set(app)?;
    let (proxy_paths, direct_paths, proxy_names, direct_names) = normalize_rules(rules);
    let route = match mode {
        ProxyMode::Full => {
            let mut rules = Vec::new();
            rules.push(json!({
                "action": "hijack-dns",
                "port": 53
            }));
            push_ru_bypass_rules(&mut rules);
            rules.push(json!({
                "inbound": [LOCAL_PROXY_TAG],
                "outbound": "proxy"
            }));
            push_process_rules(&mut rules, &direct_paths, &direct_names, "direct");
            json!({
                "rules": rules,
                "final": "proxy",
                "auto_detect_interface": true,
                "rule_set": [geoip_ru_rule_set]
            })
        }
        ProxyMode::Selected => {
            let mut rules = Vec::new();
            rules.push(json!({
                "action": "hijack-dns",
                "port": 53
            }));
            push_ru_bypass_rules(&mut rules);
            rules.push(json!({
                "inbound": [LOCAL_PROXY_TAG],
                "outbound": "proxy"
            }));
            push_process_rules(&mut rules, &direct_paths, &direct_names, "direct");
            push_process_rules(&mut rules, &proxy_paths, &proxy_names, "proxy");
            json!({
                "rules": rules,
                "final": "direct",
                "auto_detect_interface": true,
                "rule_set": [geoip_ru_rule_set]
            })
        }
        ProxyMode::Off => json!({}),
    };

    if mode != ProxyMode::Off {
        profile_obj.insert("route".to_string(), route);
    }

    let config_path = resolve_config_path(app)?;
    let content =
        serde_json::to_string_pretty(&profile).map_err(|e| err("CONFIG_INVALID", e.to_string()))?;
    fs::write(&config_path, content).map_err(|e| err("CONFIG_INVALID", e.to_string()))?;

    Ok(config_path)
}

fn refresh_state(state: &mut ProxyState) {
    if let Some(child) = state.child.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                state.last_exit = status.code();
                state.child = None;
                state.mode = ProxyMode::Off;
            }
            Ok(None) => {}
            Err(err) => {
                state.last_exit = Some(-1);
                state.last_error = Some(err.to_string());
                state.child = None;
                state.mode = ProxyMode::Off;
            }
        }
    }
}

fn spawn_monitor(app: AppHandle, state: SharedState, token: u64) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(750));
        let exit_code = {
            let mut guard = state.lock().expect("state lock");
            if guard.watch_token != token {
                return;
            }
            if let Some(child) = guard.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        guard.last_exit = status.code();
                        guard.child = None;
                        guard.mode = ProxyMode::Off;
                        guard.last_error = None;
                        status.code()
                    }
                    Ok(None) => None,
                    Err(err) => {
                        guard.last_exit = Some(-1);
                        guard.last_error = Some(err.to_string());
                        guard.child = None;
                        guard.mode = ProxyMode::Off;
                        Some(-1)
                    }
                }
            } else {
                return;
            }
        };

        if exit_code.is_some() {
            let _ = app.emit(
                "proxy-exited",
                ProxyExitPayload { code: exit_code },
            );
            return;
        }
    });
}

fn spawn_log_tailer(app: AppHandle, state: SharedState, token: u64, log_path: PathBuf) {
    std::thread::spawn(move || {
        let mut reader = match open_log_reader(&log_path) {
            Some(reader) => reader,
            None => return,
        };

        let mut pending: Vec<String> = Vec::new();
        let mut last_emit = Instant::now();
        let mut last_trim = Instant::now();

        loop {
            std::thread::sleep(Duration::from_millis(200));
            let stop = {
                let guard = match state.lock() {
                    Ok(guard) => guard,
                    Err(_) => return,
                };
                guard.watch_token != token
            };
            if stop {
                return;
            }

            if last_trim.elapsed() >= Duration::from_secs(2) {
                if trim_log_file(&log_path, LOG_KEEP_BYTES, LOG_MAX_BYTES).unwrap_or(false) {
                    if let Some(new_reader) = open_log_reader(&log_path) {
                        reader = new_reader;
                    }
                }
                last_trim = Instant::now();
            }

            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim_end_matches(['\r', '\n']);
                        if !trimmed.is_empty() {
                            pending.push(trimmed.to_string());
                        }
                    }
                    Err(_) => return,
                }
            }

            if !pending.is_empty()
                && (pending.len() >= 50 || last_emit.elapsed() >= Duration::from_millis(250))
            {
                let payload = LogBatchPayload {
                    lines: pending.drain(..).collect(),
                };
                let _ = app.emit("proxy-log-batch", payload);
                last_emit = Instant::now();
            }
        }
    });
}

fn open_log_reader(path: &PathBuf) -> Option<BufReader<std::fs::File>> {
    let file = OpenOptions::new().read(true).open(path).ok()?;
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::End(0)).is_err() {
        return None;
    }
    Some(reader)
}

fn trim_log_file(path: &PathBuf, keep_bytes: u64, max_bytes: u64) -> Result<bool, String> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return Ok(false),
    };
    let len = meta.len();
    if len <= max_bytes {
        return Ok(false);
    }
    let keep = keep_bytes.min(len);
    let start = len.saturating_sub(keep);
    let mut file = fs::File::open(path).map_err(|e| err("LOG_ERROR", e.to_string()))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|e| err("LOG_ERROR", e.to_string()))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| err("LOG_ERROR", e.to_string()))?;

    let mut out = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| err("LOG_ERROR", e.to_string()))?;
    out.write_all(&buf)
        .map_err(|e| err("LOG_ERROR", e.to_string()))?;
    Ok(true)
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn current_status(app: &AppHandle, state: &mut ProxyState) -> ProxyStatus {
    refresh_state(state);
    let profile_path = resolve_profile_path(app)
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let config_path = resolve_config_path(app)
        .ok()
        .and_then(|path| path.exists().then(|| path.display().to_string()));
    let log_path = resolve_log_path(app)
        .ok()
        .and_then(|path| path.exists().then(|| path.display().to_string()));
    let pid = state.child.as_ref().map(|child| child.id());

    ProxyStatus {
        running: state.child.is_some(),
        mode: state.mode,
        pid,
        last_exit: state.last_exit,
        last_error: state.last_error.clone(),
        config_path,
        profile_path,
        log_path,
    }
}

fn list_running_processes() -> Vec<ProcessEntry> {
    let system = System::new_with_specifics(
        RefreshKind::new().with_processes(
            ProcessRefreshKind::new().with_exe(UpdateKind::OnlyIfNotSet),
        ),
    );

    let mut entries: HashMap<String, ProcessEntry> = HashMap::new();
    for (pid, process) in system.processes() {
        let Some(path) = process.exe().and_then(|value| value.to_str()) else {
            continue;
        };
        let lower = path.to_lowercase();
        if !lower.ends_with(".exe") {
            continue;
        }
        let name = process.name().to_string();
        let entry = entries.entry(path.to_string()).or_insert_with(|| {
            let fallback = PathBuf::from(path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(path)
                .to_string();
            ProcessEntry {
                name: if name.is_empty() { fallback } else { name.clone() },
                path: path.to_string(),
                count: 0,
                pids: Vec::new(),
            }
        });
        if entry.name.is_empty() && !name.is_empty() {
            entry.name = name.clone();
        }
        entry.count += 1;
        entry.pids.push(pid.as_u32());
    }

    let mut list: Vec<ProcessEntry> = entries.into_values().collect();
    list.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    list
}

fn add_padding(value: &str) -> String {
    let remainder = value.len() % 4;
    if remainder == 0 {
        value.to_string()
    } else {
        format!("{value}{}", "=".repeat(4 - remainder))
    }
}

fn decode_base64_to_string(input: &str) -> Result<String, String> {
    let cleaned = input.trim();
    let candidates = vec![
        cleaned.to_string(),
        cleaned.replace('-', "+").replace('_', "/"),
    ];
    for candidate in candidates {
        let padded = add_padding(&candidate);
        for engine in [URL_SAFE_NO_PAD, URL_SAFE, STANDARD_NO_PAD, STANDARD] {
            if let Ok(bytes) = engine.decode(candidate.as_bytes()) {
                if let Ok(value) = String::from_utf8(bytes) {
                    return Ok(value);
                }
            }
            if candidate != padded {
                if let Ok(bytes) = engine.decode(padded.as_bytes()) {
                    if let Ok(value) = String::from_utf8(bytes) {
                        return Ok(value);
                    }
                }
            }
        }
    }
    Err(err("IMPORT_INVALID", "base64 decode failed"))
}

fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect()
}

fn decode_query_component(value: &str) -> String {
    let replaced = value.replace('+', " ");
    percent_decode_str(&replaced)
        .decode_utf8_lossy()
        .into_owned()
}

fn parse_ss_query(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        let key = decode_query_component(key).to_lowercase();
        let value = decode_query_component(value);
        params.insert(key, value);
    }
    params
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn tls_from_params(params: &HashMap<String, String>, fallback_sni: Option<String>) -> Option<Value> {
    let security = params
        .get("security")
        .or_else(|| params.get("tls"))
        .map(|value| value.to_lowercase())
        .unwrap_or_default();

    if security.is_empty() || security == "none" {
        return None;
    }

    let mut tls = json!({
        "enabled": true
    });

    if let Some(sni) = params
        .get("sni")
        .cloned()
        .or(fallback_sni)
        .or_else(|| params.get("server_name").cloned())
    {
        tls["server_name"] = json!(sni);
    }

    if let Some(insecure) = params.get("insecure") {
        let enabled = insecure == "1" || insecure.eq_ignore_ascii_case("true");
        if enabled {
            tls["insecure"] = json!(true);
        }
    }

    if let Some(alpn) = params.get("alpn") {
        let list = split_csv(alpn);
        if !list.is_empty() {
            tls["alpn"] = json!(list);
        }
    }

    if let Some(fp) = params
        .get("fp")
        .or_else(|| params.get("fingerprint"))
        .map(|value| value.trim())
    {
        if !fp.is_empty() && !fp.eq_ignore_ascii_case("none") {
            tls["utls"] = json!({
                "enabled": true,
                "fingerprint": fp
            });
        }
    }

    if security == "reality" {
        let mut reality = json!({
            "enabled": true
        });
        let mut has_reality = false;

        if let Some(pbk) = params
            .get("pbk")
            .or_else(|| params.get("public_key"))
            .or_else(|| params.get("publickey"))
        {
            if !pbk.is_empty() {
                reality["public_key"] = json!(pbk);
                has_reality = true;
            }
        }

        if params.contains_key("sid") {
            if let Some(sid) = params.get("sid") {
                reality["short_id"] = json!(sid);
                has_reality = true;
            }
        } else if params.contains_key("short_id") {
            if let Some(sid) = params.get("short_id") {
                reality["short_id"] = json!(sid);
                has_reality = true;
            }
        } else if params.contains_key("shortid") {
            if let Some(sid) = params.get("shortid") {
                reality["short_id"] = json!(sid);
                has_reality = true;
            }
        }

        if has_reality {
            tls["reality"] = reality;
        }
    }

    Some(tls)
}

fn build_transport(params: &HashMap<String, String>, network: &str) -> Option<Value> {
    match network {
        "ws" => {
            let mut transport = json!({
                "type": "ws"
            });
            if let Some(path) = params.get("path") {
                transport["path"] = json!(path);
            }
            if let Some(host) = params.get("host") {
                transport["headers"] = json!({ "Host": host });
            }
            Some(transport)
        }
        "http" | "h2" => {
            let mut transport = json!({
                "type": "http"
            });
            if let Some(host) = params.get("host") {
                let hosts = split_csv(host);
                if !hosts.is_empty() {
                    transport["host"] = json!(hosts);
                }
            }
            if let Some(path) = params.get("path") {
                transport["path"] = json!(path);
            }
            Some(transport)
        }
        "httpupgrade" => {
            let mut transport = json!({
                "type": "httpupgrade"
            });
            if let Some(host) = params.get("host") {
                transport["host"] = json!(host);
            }
            if let Some(path) = params.get("path") {
                transport["path"] = json!(path);
            }
            Some(transport)
        }
        "grpc" => {
            let mut transport = json!({
                "type": "grpc"
            });
            if let Some(service) = params
                .get("service_name")
                .or_else(|| params.get("servicename"))
                .or_else(|| params.get("path"))
            {
                transport["service_name"] = json!(service);
            }
            Some(transport)
        }
        "quic" => Some(json!({
            "type": "quic"
        })),
        _ => None,
    }
}

fn unique_tag(base: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = base.to_string();
    let mut index = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}-{index}");
        index += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn guess_tag(raw: &Value, fallback: &str) -> String {
    raw.get("tag")
        .and_then(Value::as_str)
        .filter(|tag| !tag.trim().is_empty())
        .map(|tag| tag.to_string())
        .or_else(|| raw.get("ps").and_then(Value::as_str).map(|tag| tag.to_string()))
        .unwrap_or_else(|| fallback.to_string())
}

fn parse_ss_userinfo(value: &str) -> Result<(String, String), String> {
    let decoded = if value.contains(':') {
        value.to_string()
    } else {
        decode_base64_to_string(value)?
    };
    let (method, password) = decoded
        .split_once(':')
        .ok_or_else(|| err("IMPORT_INVALID", "missing method/password"))?;
    Ok((method.to_string(), password.to_string()))
}

fn parse_ss_host_port(value: &str) -> Result<(String, u16), String> {
    let trimmed = value.trim();
    let host_port = trimmed
        .split_once('/')
        .map(|(head, _)| head)
        .unwrap_or(trimmed);

    if host_port.starts_with('[') {
        let end = host_port
            .find(']')
            .ok_or_else(|| err("IMPORT_INVALID", "invalid ipv6 host"))?;
        let host = &host_port[1..end];
        let port_str = host_port[end + 1..]
            .strip_prefix(':')
            .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
        let port = port_str
            .parse::<u16>()
            .map_err(|_| err("IMPORT_INVALID", "invalid port number"))?;
        return Ok((host.to_string(), port));
    }

    let (host, port_str) = host_port
        .rsplit_once(':')
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    if host.is_empty() {
        return Err(err("IMPORT_INVALID", "missing server"));
    }
    let port = port_str
        .parse::<u16>()
        .map_err(|_| err("IMPORT_INVALID", "invalid port number"))?;
    Ok((host.to_string(), port))
}

fn parse_ss_payload(value: &str) -> Result<(String, String, String, u16), String> {
    if let Some(at_pos) = value.rfind('@') {
        let (userinfo, hostpart) = value.split_at(at_pos);
        let hostpart = &hostpart[1..];
        let (method, password) = parse_ss_userinfo(userinfo)?;
        let (server, port) = parse_ss_host_port(hostpart)?;
        return Ok((method, password, server, port));
    }

    let decoded = decode_base64_to_string(value)?;
    if let Some(at_pos) = decoded.rfind('@') {
        let (userinfo, hostpart) = decoded.split_at(at_pos);
        let hostpart = &hostpart[1..];
        let (method, password) = parse_ss_userinfo(userinfo)?;
        let (server, port) = parse_ss_host_port(hostpart)?;
        return Ok((method, password, server, port));
    }

    Err(err("IMPORT_INVALID", "missing server"))
}

fn parse_ss(link: &str) -> Result<Value, String> {
    let raw = link.trim().trim_start_matches("ss://");
    let (payload, fragment) = raw.split_once('#').unwrap_or((raw, ""));
    let (payload, query) = payload.split_once('?').unwrap_or((payload, ""));
    let (method, password, server, port) = parse_ss_payload(payload)?;

    let mut tag = fragment.to_string();
    let params = if query.is_empty() {
        HashMap::new()
    } else {
        parse_ss_query(query)
    };
    if tag.is_empty() {
        if let Some(name) = params.get("name") {
            tag = name.to_string();
        }
    }
    let tag = if tag.is_empty() {
        format!("ss-{server}:{port}")
    } else {
        tag
    };

    let mut outbound = json!({
        "type": "shadowsocks",
        "tag": tag,
        "server": server,
        "server_port": port,
        "method": method,
        "password": password
    });

    if let Some(plugin) = params.get("plugin") {
        let mut parts = plugin.split(';');
        if let Some(plugin_name) = parts.next() {
            if !plugin_name.is_empty() {
                outbound["plugin"] = json!(plugin_name);
            }
        }
        let opts: Vec<&str> = parts.filter(|item| !item.is_empty()).collect();
        if !opts.is_empty() {
            outbound["plugin_opts"] = json!(opts.join(";"));
        }
    }

    Ok(outbound)
}

fn parse_vmess(link: &str) -> Result<Value, String> {
    let encoded = link.trim().trim_start_matches("vmess://");
    let decoded = decode_base64_to_string(encoded)?;
    let raw: Value =
        serde_json::from_str(&decoded).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let obj = raw
        .as_object()
        .ok_or_else(|| err("IMPORT_INVALID", "invalid vmess json"))?;

    let server = obj
        .get("add")
        .and_then(Value::as_str)
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = obj
        .get("port")
        .and_then(|value| {
            value
                .as_str()
                .and_then(|s| s.parse::<u16>().ok())
                .or_else(|| value.as_u64().map(|v| v as u16))
        })
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let uuid = obj
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| err("IMPORT_INVALID", "missing uuid"))?;

    let mut params: HashMap<String, String> = HashMap::new();
    for key in ["net", "type", "host", "path", "tls", "sni", "alpn"] {
        if let Some(value) = obj.get(key).and_then(Value::as_str) {
            params.insert(key.to_string(), value.to_string());
        }
    }

    let ps = obj.get("ps").and_then(Value::as_str).unwrap_or("");
    let tag = if ps.trim().is_empty() {
        format!("vmess-{server}:{port}")
    } else {
        ps.to_string()
    };

    let mut outbound = json!({
        "type": "vmess",
        "tag": tag,
        "server": server,
        "server_port": port,
        "uuid": uuid
    });

    if let Some(security) = obj
        .get("scy")
        .and_then(Value::as_str)
        .or_else(|| obj.get("security").and_then(Value::as_str))
    {
        outbound["security"] = json!(security);
    }

    if let Some(alter_id) = obj
        .get("aid")
        .and_then(|value| {
            value
                .as_str()
                .and_then(|s| s.parse::<u32>().ok())
                .or_else(|| value.as_u64().map(|v| v as u32))
        })
    {
        outbound["alter_id"] = json!(alter_id);
    }

    let network = params
        .get("net")
        .cloned()
        .unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = build_transport(&params, network.as_str()) {
        outbound["transport"] = transport;
    }

    let mut tls_params = params.clone();
    if !tls_params.contains_key("security") && !tls_params.contains_key("tls") {
        tls_params.insert("security".to_string(), "tls".to_string());
    }
    if let Some(tls) = tls_from_params(&tls_params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_vless(link: &str) -> Result<Value, String> {
    let url = Url::parse(link).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let uuid = url.username();
    if uuid.is_empty() {
        return Err(err("IMPORT_INVALID", "missing uuid"));
    }
    let server = url
        .host_str()
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = url
        .port()
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let tag = url.fragment().unwrap_or("");
    let tag = if tag.is_empty() {
        format!("vless-{server}:{port}")
    } else {
        tag.to_string()
    };
    let params = query_map(&url);

    let mut outbound = json!({
        "type": "vless",
        "tag": tag,
        "server": server,
        "server_port": port,
        "uuid": uuid
    });

    if let Some(flow) = params.get("flow") {
        outbound["flow"] = json!(flow);
    }

    let network = params
        .get("type")
        .cloned()
        .unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = build_transport(&params, network.as_str()) {
        outbound["transport"] = transport;
    }

    let mut tls_params = params.clone();
    if !tls_params.contains_key("security") && !tls_params.contains_key("tls") {
        tls_params.insert("security".to_string(), "tls".to_string());
    }
    if let Some(tls) = tls_from_params(&tls_params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_trojan(link: &str) -> Result<Value, String> {
    let url = Url::parse(link).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let server = url
        .host_str()
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = url
        .port()
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let mut password = url.username().to_string();
    if password.is_empty() {
        if let Some(pass) = url.password() {
            password = pass.to_string();
        }
    } else if let Some(pass) = url.password() {
        password = format!("{password}:{pass}");
    }

    if password.is_empty() {
        return Err(err("IMPORT_INVALID", "missing password"));
    }
    let tag = url.fragment().unwrap_or("");
    let tag = if tag.is_empty() {
        format!("trojan-{server}:{port}")
    } else {
        tag.to_string()
    };
    let params = query_map(&url);

    let mut outbound = json!({
        "type": "trojan",
        "tag": tag,
        "server": server,
        "server_port": port,
        "password": password
    });

    let network = params
        .get("type")
        .cloned()
        .unwrap_or_else(|| "tcp".to_string());
    if let Some(transport) = build_transport(&params, network.as_str()) {
        outbound["transport"] = transport;
    }

    if let Some(tls) = tls_from_params(&params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_hysteria(link: &str) -> Result<Value, String> {
    let url = Url::parse(link).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let server = url
        .host_str()
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = url
        .port()
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let params = query_map(&url);
    let tag = url.fragment().unwrap_or("");
    let tag = if tag.is_empty() {
        format!("hysteria-{server}:{port}")
    } else {
        tag.to_string()
    };

    let mut outbound = json!({
        "type": "hysteria",
        "tag": tag,
        "server": server,
        "server_port": port
    });

    if let Some(auth) = params.get("auth").or_else(|| params.get("auth_str")) {
        outbound["auth_str"] = json!(auth);
    }
    if let Some(obfs) = params.get("obfs") {
        outbound["obfs"] = json!(obfs);
    }
    if let Some(up) = params.get("upmbps").or_else(|| params.get("up")) {
        if let Ok(value) = up.parse::<u32>() {
            outbound["up_mbps"] = json!(value);
        }
    }
    if let Some(down) = params.get("downmbps").or_else(|| params.get("down")) {
        if let Ok(value) = down.parse::<u32>() {
            outbound["down_mbps"] = json!(value);
        }
    }

    let mut tls_params = params.clone();
    if !tls_params.contains_key("security") {
        tls_params.insert("security".to_string(), "tls".to_string());
    }
    if let Some(peer) = params.get("peer") {
        tls_params.insert("sni".to_string(), peer.to_string());
    }
    if let Some(tls) = tls_from_params(&tls_params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_hysteria2(link: &str) -> Result<Value, String> {
    let url = Url::parse(link).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let server = url
        .host_str()
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = url
        .port()
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let password = url.username();
    if password.is_empty() {
        return Err(err("IMPORT_INVALID", "missing password"));
    }
    let params = query_map(&url);
    let tag = url.fragment().unwrap_or("");
    let tag = if tag.is_empty() {
        format!("hysteria2-{server}:{port}")
    } else {
        tag.to_string()
    };

    let mut outbound = json!({
        "type": "hysteria2",
        "tag": tag,
        "server": server,
        "server_port": port,
        "password": password
    });

    if let Some(obfs) = params.get("obfs") {
        if obfs == "salamander" {
            let mut obfs_obj = json!({
                "type": "salamander"
            });
            if let Some(password) = params
                .get("obfs-password")
                .or_else(|| params.get("obfs_password"))
            {
                obfs_obj["password"] = json!(password);
            }
            outbound["obfs"] = obfs_obj;
        }
    }

    if let Some(up) = params.get("upmbps").or_else(|| params.get("up")) {
        if let Ok(value) = up.parse::<u32>() {
            outbound["up_mbps"] = json!(value);
        }
    }
    if let Some(down) = params.get("downmbps").or_else(|| params.get("down")) {
        if let Ok(value) = down.parse::<u32>() {
            outbound["down_mbps"] = json!(value);
        }
    }

    let mut tls_params = params.clone();
    if !tls_params.contains_key("security") {
        tls_params.insert("security".to_string(), "tls".to_string());
    }
    if let Some(tls) = tls_from_params(&tls_params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_tuic(link: &str) -> Result<Value, String> {
    let url = Url::parse(link).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let server = url
        .host_str()
        .ok_or_else(|| err("IMPORT_INVALID", "missing server"))?;
    let port = url
        .port()
        .ok_or_else(|| err("IMPORT_INVALID", "missing port"))?;
    let mut uuid = url.username().to_string();
    let mut password = url.password().unwrap_or("").to_string();
    if password.is_empty() {
        if let Some((left, right)) = uuid.split_once(':') {
            let left = left.to_string();
            let right = right.to_string();
            uuid = left;
            password = right;
        }
    }
    if uuid.is_empty() || password.is_empty() {
        return Err(err("IMPORT_INVALID", "missing uuid/password"));
    }
    let params = query_map(&url);
    let tag = url.fragment().unwrap_or("");
    let tag = if tag.is_empty() {
        format!("tuic-{server}:{port}")
    } else {
        tag.to_string()
    };

    let mut outbound = json!({
        "type": "tuic",
        "tag": tag,
        "server": server,
        "server_port": port,
        "uuid": uuid,
        "password": password
    });

    if let Some(congestion) = params.get("congestion_control") {
        outbound["congestion_control"] = json!(congestion);
    }
    if let Some(udp_mode) = params.get("udp_relay_mode") {
        outbound["udp_relay_mode"] = json!(udp_mode);
    }

    if let Some(tls) = tls_from_params(&params, Some(server.to_string())) {
        outbound["tls"] = tls;
    }

    Ok(outbound)
}

fn parse_share_link(link: &str) -> Result<Value, String> {
    let trimmed = link.trim();
    if trimmed.starts_with("ss://") {
        return parse_ss(trimmed);
    }
    if trimmed.starts_with("vmess://") {
        return parse_vmess(trimmed);
    }
    if trimmed.starts_with("vless://") {
        return parse_vless(trimmed);
    }
    if trimmed.starts_with("trojan://") {
        return parse_trojan(trimmed);
    }
    if trimmed.starts_with("hysteria2://") || trimmed.starts_with("hy2://") {
        return parse_hysteria2(trimmed);
    }
    if trimmed.starts_with("hysteria://") {
        return parse_hysteria(trimmed);
    }
    if trimmed.starts_with("tuic://") {
        return parse_tuic(trimmed);
    }
    Err(err("IMPORT_UNSUPPORTED", "unsupported share link"))
}

fn append_outbounds(app: &AppHandle, mut new_outbounds: Vec<Value>) -> Result<ImportResult, String> {
    let mut profile = load_profile_json(app)?;
    let profile_obj = profile
        .as_object_mut()
        .ok_or_else(|| err("PROFILE_INVALID", "root must be an object"))?;
    let existing_outbounds = profile_obj
        .get("outbounds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut outbounds = existing_outbounds;
    let mut used_tags: HashSet<String> = outbounds
        .iter()
        .filter_map(|item| item.get("tag").and_then(Value::as_str))
        .map(|tag| tag.to_string())
        .collect();

    let mut added = 0;
    let mut errors = Vec::new();
    let mut first_added: Option<String> = None;
    for outbound in new_outbounds.drain(..) {
        let Some(obj) = outbound.as_object() else {
            errors.push("Invalid outbound object".to_string());
            continue;
        };

        let fallback = obj
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("profile");
        let tag = guess_tag(&outbound, fallback);
        let unique = unique_tag(&tag, &mut used_tags);
        let mut outbound = outbound;
        outbound["tag"] = json!(unique.clone());
        if first_added.is_none() {
            first_added = Some(unique.clone());
        }
        outbounds.push(outbound);
        added += 1;
    }

    profile_obj.insert("outbounds".to_string(), Value::Array(outbounds));
    save_profile_json(app, &profile)?;

    let mut state = load_profile_state(app);
    if state.active_tag.is_none() {
        if let Some(tag) = first_added {
            state.active_tag = Some(tag);
            let _ = save_profile_state(app, &state);
        }
    }

    Ok(ImportResult {
        profile: profile_data(app, &profile),
        added,
        errors,
    })
}

#[tauri::command]
fn get_status(app: AppHandle, state: State<SharedState>) -> ProxyStatus {
    let mut guard = state.lock().expect("state lock");
    current_status(&app, &mut guard)
}

#[tauri::command]
fn get_saved_state(app: AppHandle) -> AppState {
    load_app_state(&app)
}

#[tauri::command]
fn list_processes() -> Vec<ProcessEntry> {
    list_running_processes()
}

#[tauri::command]
fn read_log_tail(app: AppHandle, limit: Option<usize>) -> Result<Vec<String>, String> {
    let limit = limit.unwrap_or(200).max(1);
    let path = resolve_log_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = fs::File::open(&path).map_err(|e| err("LOG_ERROR", e.to_string()))?;
    let file_len = file
        .metadata()
        .map_err(|e| err("LOG_ERROR", e.to_string()))?
        .len();
    if file_len == 0 {
        return Ok(Vec::new());
    }

    let mut read_size: u64 = 64 * 1024;
    let mut lines: Vec<String> = Vec::new();
    loop {
        let start = if file_len > read_size {
            file_len - read_size
        } else {
            0
        };
        file.seek(SeekFrom::Start(start))
            .map_err(|e| err("LOG_ERROR", e.to_string()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| err("LOG_ERROR", e.to_string()))?;
        let text = String::from_utf8_lossy(&buf);
        lines.clear();
        lines.extend(text.lines().map(|line| line.to_string()));
        if lines.len() >= limit || start == 0 {
            break;
        }
        read_size = (read_size * 2).min(file_len);
    }

    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

#[tauri::command]
fn apply_mode(
    app: &AppHandle,
    state: &SharedState,
    mode: ProxyMode,
    app_rules: Vec<AppRule>,
) -> Result<ProxyStatus, String> {
    let _ = save_app_state(
        app,
        &AppState {
            last_mode: mode,
            app_rules: app_rules.clone(),
        },
    );

    let mut guard = state.lock().expect("state lock");

    if let Some(mut child) = guard.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }

    guard.mode = ProxyMode::Off;
    guard.last_error = None;

    if mode == ProxyMode::Off {
        guard.watch_token = guard.watch_token.wrapping_add(1);
        return Ok(current_status(app, &mut guard));
    }

    let config_path = match build_config(app, mode, app_rules) {
        Ok(path) => path,
        Err(err) => {
            guard.last_error = Some(err.clone());
            return Err(err);
        }
    };
    let log_path = resolve_log_path(app)?;
    let exe_path = match ensure_singbox_exe(app) {
        Ok(path) => path,
        Err(err) => {
            guard.last_error = Some(err.clone());
            return Err(err);
        }
    };

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| {
            let message = err("LOG_ERROR", e.to_string());
            guard.last_error = Some(message.clone());
            message
        })?;

    let mut cmd = Command::new(exe_path);
    cmd.arg("run").arg("-c").arg(&config_path);
    cmd.stdout(Stdio::from(
        log_file
            .try_clone()
            .map_err(|e| err("LOG_ERROR", e.to_string()))?,
    ));
    cmd.stderr(Stdio::from(log_file));

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd.spawn().map_err(|e| {
        let message = err("START_FAILED", e.to_string());
        guard.last_error = Some(message.clone());
        message
    })?;

    #[cfg(target_os = "windows")]
    {
        if guard.job.is_none() {
            if let Ok(job) = create_job_object() {
                guard.job = Some(job);
            }
        }
        if let Some(job) = guard.job.as_ref() {
            let _ = unsafe { AssignProcessToJobObject(job.0, child.as_raw_handle() as isize) };
        }
    }

    guard.child = Some(child);
    guard.mode = mode;
    guard.config_path = Some(config_path);
    guard.last_exit = None;

    guard.watch_token = guard.watch_token.wrapping_add(1);
    let token = guard.watch_token;
    let state_clone = state.clone();
    spawn_monitor(app.clone(), state_clone, token);
    let log_state = state.clone();
    spawn_log_tailer(app.clone(), log_state, token, log_path);

    Ok(current_status(app, &mut guard))
}

#[tauri::command]
fn set_mode(
    app: AppHandle,
    state: State<SharedState>,
    mode: ProxyMode,
    app_rules: Vec<AppRule>,
) -> Result<ProxyStatus, String> {
    apply_mode(&app, state.inner(), mode, app_rules)
}

#[tauri::command]
fn get_profiles(app: AppHandle) -> Result<ProfileData, String> {
    let profile = load_profile_json(&app)?;
    Ok(profile_data(&app, &profile))
}

#[tauri::command]
fn set_active_profile(app: AppHandle, tag: String) -> Result<ProfileData, String> {
    let mut state = load_profile_state(&app);
    state.active_tag = Some(tag);
    save_profile_state(&app, &state)?;
    let profile = load_profile_json(&app)?;
    Ok(profile_data(&app, &profile))
}

#[tauri::command]
fn remove_outbound(app: AppHandle, tag: String) -> Result<ProfileData, String> {
    let mut profile = load_profile_json(&app)?;
    let profile_obj = profile
        .as_object_mut()
        .ok_or_else(|| err("PROFILE_INVALID", "root must be an object"))?;
    let outbounds = profile_obj
        .get("outbounds")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let filtered: Vec<Value> = outbounds
        .into_iter()
        .filter(|item| item.get("tag").and_then(Value::as_str) != Some(tag.as_str()))
        .collect();
    profile_obj.insert("outbounds".to_string(), Value::Array(filtered));
    save_profile_json(&app, &profile)?;

    let mut state = load_profile_state(&app);
    if state.active_tag.as_deref() == Some(tag.as_str()) {
        state.active_tag = None;
        let _ = save_profile_state(&app, &state);
    }
    Ok(profile_data(&app, &profile))
}

#[tauri::command]
fn import_share_links(app: AppHandle, links: Vec<String>) -> Result<ImportResult, String> {
    let mut errors = Vec::new();
    let mut outbounds = Vec::new();
    for link in links {
        if link.trim().is_empty() {
            continue;
        }
        match parse_share_link(link.as_str()) {
            Ok(outbound) => outbounds.push(outbound),
            Err(error) => errors.push(format!("{link}: {error}")),
        }
    }

    if outbounds.is_empty() {
        return Err(err(
            "IMPORT_FAILED",
            if errors.is_empty() {
                "no valid links".to_string()
            } else {
                errors.join("\n")
            },
        ));
    }

    let mut result = append_outbounds(&app, outbounds)?;
    result.errors.extend(errors);
    Ok(result)
}

#[tauri::command]
fn import_outbound_json(app: AppHandle, payload: String) -> Result<ImportResult, String> {
    let value: Value =
        serde_json::from_str(&payload).map_err(|e| err("IMPORT_INVALID", e.to_string()))?;
    let mut outbounds = Vec::new();
    match value {
        Value::Array(values) => {
            outbounds.extend(values);
        }
        Value::Object(map) => {
            if let Some(inner) = map.get("outbounds") {
                if let Some(arr) = inner.as_array() {
                    outbounds.extend(arr.clone());
                }
            } else {
                outbounds.push(Value::Object(map));
            }
        }
        _ => {
            return Err(err("IMPORT_INVALID", "unsupported JSON format"));
        }
    }
    if outbounds.is_empty() {
        return Err(err("IMPORT_INVALID", "no outbounds found"));
    }
    append_outbounds(&app, outbounds)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let autostart_launch = std::env::args().any(|arg| arg == AUTOSTART_ARG);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ExitFlag::default())
        .manage(Arc::new(Mutex::new(ProxyState::default())))
        .setup(move |app| {
            let app_handle = app.handle();
            let saved_state = load_app_state(&app_handle);
            let saved_mode = saved_state.last_mode;
            let saved_rules = saved_state.app_rules;

            let tray_menu = Menu::new(app)?;
            let open_item = MenuItemBuilder::with_id(TRAY_OPEN_ID, "Открыть").build(app)?;
            let exit_item = MenuItemBuilder::with_id(TRAY_EXIT_ID, "Закрыть").build(app)?;
            tray_menu.append_items(&[&open_item, &exit_item])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("YotsubaCore")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    TRAY_OPEN_ID => show_main_window(app),
                    TRAY_EXIT_ID => {
                        let exit_flag = app.state::<ExitFlag>();
                        exit_flag.allow_exit();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let tray = tray_builder.build(app)?;
            app.manage(TrayState { _tray: tray });

            if autostart_launch && saved_mode != ProxyMode::Off {
                hide_main_window(&app_handle);
            }

            let state = app.state::<SharedState>();
            let _ = apply_mode(&app_handle, state.inner(), saved_mode, saved_rules);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_saved_state,
            list_processes,
            read_log_tail,
            set_mode,
            get_profiles,
            set_active_profile,
            remove_outbound,
            import_share_links,
            import_outbound_json
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        match event {
            tauri::RunEvent::WindowEvent { label, event, .. } => {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let exit_flag = app_handle.state::<ExitFlag>();
                    if exit_flag.is_allowed() {
                        return;
                    }
                    api.prevent_close();
                    if label == "main" {
                        hide_main_window(app_handle);
                    }
                }
            }
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                let state: State<SharedState> = app_handle.state();
                let guard_result = state.lock();
                if let Ok(mut guard) = guard_result {
                    if let Some(mut child) = guard.child.take() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
            _ => {}
        }
    });
}
