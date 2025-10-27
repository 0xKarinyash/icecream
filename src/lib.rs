#![allow(non_camel_case_types)] 
#![allow(non_snake_case)] 

use ctor::ctor;
use ini::ini;
use libloading::{Library, Symbol};
use once_cell::sync::Lazy;
use std::error::Error;
use std::ffi::c_void;
use std::ffi::CStr;
use std::io::prelude::*;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;
use std::collections::HashMap;
use std::ptr;
use std::mem;
use chrono::Local;

const CONFIG_PATH: &str = "icecream.ini";
const LOGGING_LEVEL: u8 = 3; // 0 -- no logs; 1 -- logs only to console; 2 -- logs only to file; 3 -- logs to both file and console

// logger
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        if LOGGING_LEVEL > 0 {
            let now = Local::now();
            let timestamp = now.format("%Y-%m-%d %H:%M:%S");

            let user_message = format!($($arg)*);
            let final_message = format!("[ICECREAM {}] {}", timestamp, user_message);

            if LOGGING_LEVEL == 1 || LOGGING_LEVEL == 3 { 
                println!("{}", final_message);
            }

            if LOGGING_LEVEL >= 2 && let Ok(home) = std::env::var("HOME") {
                let log_path = format!("{}/icecream_log.txt", home);
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(log_path)
                {
                    let _ = file.write_all(format!("{}\n", final_message).as_bytes());
                }
            }
        }
    }};
}

pub struct DlcInfo {
    pub id: u32,
    pub name: String,
}

static DLCS: Lazy<Mutex<Vec<DlcInfo>>> = Lazy::new(|| {
    Mutex::new(Vec::new())
});

static ORIGINAL_FUNCTIONS: Lazy<Mutex<HashMap<String, usize>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

static PATCHED_VTABLES: Lazy<Mutex<HashMap<usize, Box<[usize]>>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

// BIsDlcInstalled
unsafe extern "C" fn hooked_b_is_dlc_installed(_this: *mut c_void, app_id: u32) -> bool {
    log!("[HOOK] BIsDlcInstalled called for AppID: {app_id}");
    let dlcs_guard = DLCS.lock().unwrap();

    if dlcs_guard.iter().any(|dlc| dlc.id == app_id) {
        log!("[HOOK] -> AppID {app_id} found in list. Unlocked!");
        return true;
    }
    
    log!("[HOOK] -> AppID {app_id} not found. Denied");
    return false;
}

// UserHasLicenseForApp
// 0 = k_EUserHasLicenseForAppResultHasLicense, 2 = k_EUserHasLicenseForAppResultDoesNotHaveLicense
unsafe extern "C" fn hooked_user_has_license(_this: *mut c_void, _steam_id: u64, app_id: u32) -> u32 {
    log!("[HOOK] UserHasLicenseForApp called for AppID: {app_id}");
    let dlcs_guard = DLCS.lock().unwrap();

    if dlcs_guard.iter().any(|dlc| dlc.id == app_id) {
        log!("[HOOK] -> AppID {app_id} found in list. Unlocked!");
        return 0; // k_EUserHasLicenseForAppResultHasLicense
    }
    
    log!("[HOOK] -> AppID {app_id} not found. Denied");
    return 2; // k_EUserHasLicenseForAppResultDoesNotHaveLicense
}

pub type HSteamUser = u32;
pub type SteamAPICall_t = u64;

#[repr(C)]
pub enum EServerMode {
    eServerModeInvalid = -1,
    eServerModeNoAuthentication = 1,
    eServerModeAuthentication = 2,
    eServerModeAuthenticationAndSecure = 3,
}

struct RealApi {
    _lib: Library,
    api_init: Symbol<'static, unsafe extern "C" fn() -> bool>,
    internal_create_interface: Symbol<'static, unsafe extern "C" fn(*const c_char) -> *mut c_void>,
    internal_find_or_create_user_interface: Symbol<'static, unsafe extern "C" fn(HSteamUser, *const c_char) -> *mut c_void>,

    is_steam_running: Symbol<'static, unsafe extern "C" fn() -> bool>,
    get_h_steam_user: Symbol<'static, unsafe extern "C" fn() -> HSteamUser>,
    register_call_result: Symbol<'static, unsafe extern "C" fn(*mut c_void, SteamAPICall_t)>,
    register_callback: Symbol<'static, unsafe extern "C" fn(*mut c_void, i32)>,
    run_callbacks: Symbol<'static, unsafe extern "C" fn()>,
    shutdown: Symbol<'static, unsafe extern "C" fn()>,
    unregister_call_result: Symbol<'static, unsafe extern "C" fn(*mut c_void, SteamAPICall_t)>,
    unregister_callback: Symbol<'static, unsafe extern "C" fn(*mut c_void)>,
    game_server_get_h_steam_user: Symbol<'static, unsafe extern "C" fn() -> HSteamUser>,
    game_server_run_callbacks: Symbol<'static, unsafe extern "C" fn()>,
    game_server_shutdown: Symbol<'static, unsafe extern "C" fn()>,
    internal_context_init: Symbol<'static, unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    internal_find_or_create_game_server_interface: Symbol<'static, unsafe extern "C" fn(HSteamUser, *const c_char) -> *mut c_void>,
    internal_game_server_init: Symbol<'static, unsafe extern "C" fn(u32, u16, u16, u16, EServerMode, *const c_char) -> bool>,
}

static REAL_STEAM_API: Lazy<Result<RealApi, libloading::Error>> = Lazy::new(|| unsafe {
    let lib = Library::new("libsteam_api_o.dylib")?;
    Ok(RealApi {
        api_init: mem::transmute(lib.get::<unsafe extern "C" fn() -> bool>(b"SteamAPI_Init")?),
        internal_create_interface: mem::transmute(lib.get::<unsafe extern "C" fn(*const c_char) -> *mut c_void>(b"SteamInternal_CreateInterface")?),
        internal_find_or_create_user_interface: mem::transmute(lib.get::<unsafe extern "C" fn(HSteamUser, *const c_char) -> *mut c_void>(b"SteamInternal_FindOrCreateUserInterface")?),
        
        is_steam_running: mem::transmute(lib.get::<unsafe extern "C" fn() -> bool>(b"SteamAPI_IsSteamRunning")?),
        get_h_steam_user: mem::transmute(lib.get::<unsafe extern "C" fn() -> HSteamUser>(b"SteamAPI_GetHSteamUser")?),
        register_call_result: mem::transmute(lib.get::<unsafe extern "C" fn(*mut c_void, SteamAPICall_t)>(b"SteamAPI_RegisterCallResult")?),
        register_callback: mem::transmute(lib.get::<unsafe extern "C" fn(*mut c_void, i32)>(b"SteamAPI_RegisterCallback")?),
        run_callbacks: mem::transmute(lib.get::<unsafe extern "C" fn()>(b"SteamAPI_RunCallbacks")?),
        shutdown: mem::transmute(lib.get::<unsafe extern "C" fn()>(b"SteamAPI_Shutdown")?),
        unregister_call_result: mem::transmute(lib.get::<unsafe extern "C" fn(*mut c_void, SteamAPICall_t)>(b"SteamAPI_UnregisterCallResult")?),
        unregister_callback: mem::transmute(lib.get::<unsafe extern "C" fn(*mut c_void)>(b"SteamAPI_UnregisterCallback")?),
        game_server_get_h_steam_user: mem::transmute(lib.get::<unsafe extern "C" fn() -> HSteamUser>(b"SteamGameServer_GetHSteamUser")?),
        game_server_run_callbacks: mem::transmute(lib.get::<unsafe extern "C" fn()>(b"SteamGameServer_RunCallbacks")?),
        game_server_shutdown: mem::transmute(lib.get::<unsafe extern "C" fn()>(b"SteamGameServer_Shutdown")?),
        internal_context_init: mem::transmute(lib.get::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(b"SteamInternal_ContextInit")?),
        internal_find_or_create_game_server_interface: mem::transmute(lib.get::<unsafe extern "C" fn(HSteamUser, *const c_char) -> *mut c_void>(b"SteamInternal_FindOrCreateGameServerInterface")?),
        internal_game_server_init: mem::transmute(lib.get::<unsafe extern "C" fn(u32, u16, u16, u16, EServerMode, *const c_char) -> bool>(b"SteamInternal_GameServer_Init")?),
        _lib: lib,
    })
});

fn load_dlcs_from_ini(config_path: &str) -> Result<u16, Box<dyn Error>>{
    if !Path::new(config_path).exists() {
        return Err("No config file found!".into())
    }
    let mut dlcs_guard = DLCS.lock().unwrap();
    let map = ini!(safe config_path);
    let mut count = 0;

    for (id, name) in map.unwrap()["dlc"].clone() {
        let id_num = id.parse::<u32>()?;
        let name_str = name.unwrap_or("Unknown".to_string());
        dlcs_guard.push(DlcInfo { id: id_num, name: name_str.clone() });
        log!("[CONFIG] Loaded {id_num} -- {name_str} from {config_path}");
        count += 1;
    }

    Ok(count)
}

#[ctor]
fn constructor() {
    log!("\n\n--- IceCream loaded ---");
    match load_dlcs_from_ini(CONFIG_PATH) {
        Ok(c) => log!("Loaded {c} dlcs from {CONFIG_PATH}"),
        Err(e) => log!("Failed to load dlcs from {CONFIG_PATH}. Nothing will be unlocked! Error: {e}")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_Init() -> bool {
    match &*REAL_STEAM_API {
        Ok(api) => {
            log!("SteamAPI_Init HOOKED. Calling original function.");
            let result = unsafe { (api.api_init)() };
            log!("Original SteamAPI_Init returned: {result}");
            return result;
        }
        Err(e) => {
            log!("SteamAPI_Init HOOKED. Failed to load real API: {e}. Faking success.");
        }
    }

    true
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamInternal_CreateInterface(version: *const c_char) -> *mut c_void {
    let version_str = unsafe { CStr::from_ptr(version).to_string_lossy().into_owned() };

    log!("SteamInternal_CreateInterface requested: {version_str}");

    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.internal_create_interface)(version) } } else { ptr::null_mut() }
}

// really messy because of indexes and idk how to make it more... durable atm
#[unsafe(no_mangle)]
pub extern "C" fn SteamInternal_FindOrCreateUserInterface(h_steam_user: HSteamUser, version: *const c_char) -> *mut c_void {
    let version_str = unsafe { CStr::from_ptr(version).to_string_lossy().into_owned() };
    log!("SteamInternal_FindOrCreateUserInterface requested: {}", version_str);

    let original_interface = if let Ok(api) = &*REAL_STEAM_API {
        unsafe { ((api).internal_find_or_create_user_interface)(h_steam_user, version) }
    } else {
        return ptr::null_mut();
    };
    
    if original_interface.is_null() {
        return original_interface;
    }

    let interface_addr = original_interface as usize;
    let mut vtables = PATCHED_VTABLES.lock().unwrap();
    if vtables.contains_key(&interface_addr) {
        return original_interface;
    }

    unsafe {
        let original_vtable_ptr = *(original_interface as *mut *mut usize);
        
        if version_str.starts_with("STEAMAPPS_INTERFACE_VERSION") {
            log!("!!! Locked ISteamApps. Patching vtable...");
            let mut new_vtable = Vec::from(std::slice::from_raw_parts(original_vtable_ptr, 200));

            let original_fn_ptr = new_vtable[7];
            ORIGINAL_FUNCTIONS.lock().unwrap().insert("BIsDlcInstalled".to_string(), original_fn_ptr);
            new_vtable[7] = hooked_b_is_dlc_installed as usize;

            let leaked_vtable = new_vtable.into_boxed_slice();
            *(original_interface as *mut *mut usize) = leaked_vtable.as_ptr() as *mut usize;
            vtables.insert(interface_addr, leaked_vtable);
        } else if version_str.starts_with("SteamUser") {
            log!("!!! Locked SteamUser. Patching vtable...");
            let mut new_vtable = Vec::from(std::slice::from_raw_parts(original_vtable_ptr, 200));

            let original_fn_ptr = new_vtable[24];
            ORIGINAL_FUNCTIONS.lock().unwrap().insert("UserHasLicenseForApp".to_string(), original_fn_ptr);
            new_vtable[24] = hooked_user_has_license as usize;

            let leaked_vtable = new_vtable.into_boxed_slice();
            *(original_interface as *mut *mut usize) = leaked_vtable.as_ptr() as *mut usize;
            vtables.insert(interface_addr, leaked_vtable);
        }
    }

    original_interface
}

// other proxied methods

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_GetHSteamUser() -> HSteamUser {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.get_h_steam_user)() } } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_IsSteamRunning() -> bool {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.is_steam_running)() } } else { true }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_RegisterCallResult(callback: *mut c_void, api_call: SteamAPICall_t) {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.register_call_result)(callback, api_call) } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_RegisterCallback(callback: *mut c_void, callback_id: i32) {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.register_callback)(callback, callback_id) } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_RunCallbacks() {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.run_callbacks)() } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_Shutdown() {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.shutdown)() } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_UnregisterCallResult(callback: *mut c_void, api_call: SteamAPICall_t) {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.unregister_call_result)(callback, api_call) } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamAPI_UnregisterCallback(callback: *mut c_void) {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.unregister_callback)(callback) } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamGameServer_GetHSteamUser() -> HSteamUser {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.game_server_get_h_steam_user)() } } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamGameServer_RunCallbacks() {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.game_server_run_callbacks)() } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamGameServer_Shutdown() {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.game_server_shutdown)() } }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamInternal_ContextInit(context: *mut c_void) -> *mut c_void {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.internal_context_init)(context) } } else { ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamInternal_FindOrCreateGameServerInterface(h_steam_user: HSteamUser, version: *const c_char) -> *mut c_void {
    if let Ok(api) = &*REAL_STEAM_API { unsafe { (api.internal_find_or_create_game_server_interface)(h_steam_user, version) } } else { ptr::null_mut() }
}

#[unsafe(no_mangle)]
pub extern "C" fn SteamInternal_GameServer_Init(
    un_ip: u32,
    us_steam_port: u16,
    us_game_port: u16,
    us_query_port: u16,
    e_server_mode: EServerMode,
    pch_version_string: *const c_char,
) -> bool {
    if let Ok(api) = &*REAL_STEAM_API {
        unsafe { (api.internal_game_server_init)(un_ip, us_steam_port, us_game_port, us_query_port, e_server_mode, pch_version_string) }
    } else {
        false
    }
}