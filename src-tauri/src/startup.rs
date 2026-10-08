use serde::{Deserialize, Serialize};
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::{AppHandle, State};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_autostart::ManagerExt;

use crate::{blocking_command, commit_app_data, lock_error, AppData, AppState};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutostartOwner {
    platform: String,
    channel: String,
    target: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    enabled: bool,
    conflict: bool,
    reason: Option<String>,
}

// Startup ownership belongs to this machine, not to imported or frontend snapshots.
pub fn preserve_local_state(candidate: &mut AppData, current: &AppData) {
    candidate.autostart_owner = current.autostart_owner.clone();
    candidate.settings.autostart = current.settings.autostart;
}

fn normalized_path(path: &Path) -> Result<String, String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let path = path.to_str().ok_or("Startup path is not valid UTF-8")?;
    #[cfg(target_os = "windows")]
    return Ok(path.trim_start_matches(r"\\?\").to_lowercase());
    #[cfg(not(target_os = "windows"))]
    Ok(path.to_string())
}

fn same_target(left: &str, right: &str) -> bool {
    match (
        normalized_path(Path::new(left)),
        normalized_path(Path::new(right)),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

// 判定当前进程是否来自已安装目录，用于区分安装包版与便携版的自启动归属。
// 逻辑随 updater 模块一并保留在此：自启动冲突检测需要区分两种运行形态。
#[cfg(target_os = "windows")]
fn is_installed(app: &AppHandle, executable: &Path) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;

    // NSIS currentUser installs record a quoted InstallLocation under the product name.
    let key = format!(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        app.package_info().name
    );
    let location = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_64KEY)
        .and_then(|key| key.get_value::<String, _>("InstallLocation"));
    location.is_ok_and(|location| matches_windows_install(executable, &location))
}

#[cfg(target_os = "windows")]
fn matches_windows_install(executable: &Path, location: &str) -> bool {
    let install_dir = Path::new(location.trim_matches('"'));
    // A copied standalone executable must not start an installer for another copy.
    match (executable.parent(), install_dir.canonicalize()) {
        (Some(parent), Ok(installed)) if installed.join("uninstall.exe").is_file() => parent
            .canonicalize()
            .is_ok_and(|parent| parent == installed),
        _ => false,
    }
}

#[cfg(target_os = "macos")]
fn is_installed(app: &AppHandle, executable: &Path) -> bool {
    let Some(bundle) = executable.ancestors().nth(3) else {
        return false;
    };
    if bundle
        .extension()
        .is_none_or(|extension| extension != "app")
    {
        return false;
    }
    // Copies still on the DMG or in App Translocation must be moved to Applications first.
    bundle.starts_with("/Applications")
        || app
            .path()
            .home_dir()
            .is_ok_and(|home| bundle.starts_with(home.join("Applications")))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_installed(_app: &AppHandle, _executable: &Path) -> bool {
    false
}

fn current_owner(app: &AppHandle) -> Result<AutostartOwner, String> {
    #[cfg(target_os = "windows")]
    if option_env!("ZHIXIE_STORE_BUILD").is_some() {
        return Ok(AutostartOwner {
            platform: "windows".into(),
            channel: "store".into(),
            target: windows::package_family()?,
        });
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    Ok(AutostartOwner {
        platform: std::env::consts::OS.into(),
        channel: if is_installed(app, &executable) {
            "installed"
        } else {
            "portable"
        }
        .into(),
        target: normalized_path(&executable)?,
    })
}

fn cross_channel(owner: &AutostartOwner, current: &AutostartOwner) -> bool {
    owner.platform == current.platform && (owner.channel == "store") != (current.channel == "store")
}

fn path_owner(
    target: String,
    current: &AutostartOwner,
    saved: Option<&AutostartOwner>,
) -> AutostartOwner {
    if same_target(&target, &current.target) {
        return current.clone();
    }
    if let Some(saved) =
        saved.filter(|owner| owner.channel != "store" && same_target(&target, &owner.target))
    {
        return saved.clone();
    }
    AutostartOwner {
        platform: current.platform.clone(),
        channel: "portable".into(),
        target,
    }
}

/// Query only: startup reconciliation must never re-enable a system-disabled item.
pub fn observe(app: &AppHandle, data: &mut AppData) -> Result<AutostartStatus, String> {
    let current = current_owner(app)?;
    let (owner, enabled, reason, native_conflict) =
        platform_status(app, &current, data.autostart_owner.as_ref())?;
    let conflict = native_conflict
        || owner
            .as_ref()
            .is_some_and(|owner| cross_channel(owner, &current));
    data.autostart_owner = owner;
    data.settings.autostart = enabled;
    Ok(AutostartStatus {
        enabled,
        conflict,
        reason,
    })
}

#[tauri::command]
pub async fn get_autostart_status(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AutostartStatus, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let mut current = state.0.data.lock().map_err(lock_error)?;
        let mut candidate = current.clone();
        let status = observe(&app, &mut candidate)?;
        if candidate.autostart_owner != current.autostart_owner
            || candidate.settings.autostart != current.settings.autostart
        {
            commit_app_data(&state.0.data_path, &mut current, candidate)?;
        }
        state
            .0
            .native_errors
            .lock()
            .map_err(lock_error)?
            .autostart_error = None;
        Ok(status)
    })
    .await
}

#[tauri::command]
pub async fn set_autostart(
    window: tauri::WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AutostartStatus, String> {
    if window.label() != "main" {
        return Err("Startup settings are only available in the main window".into());
    }
    let state = state.inner().clone();
    blocking_command(move || {
        let mut current = state.0.data.lock().map_err(lock_error)?;
        let mut candidate = current.clone();
        let status = observe(&app, &mut candidate)?;
        if enabled && status.conflict {
            return Err("conflict".into());
        }
        let owner = current_owner(&app)?;
        if enabled {
            validate_startup_path(&owner)?;
        }
        // A disabled switch for another copy must not remove its startup registration.
        if !enabled && !status.enabled {
            commit_app_data(&state.0.data_path, &mut current, candidate)?;
            return Ok(status);
        }
        let snapshot = snapshot(&app, &owner)?;
        with_rollback(
            || {
                change_native(&app, &owner, enabled)?;
                let status = observe(&app, &mut candidate)?;
                if status.enabled != enabled {
                    return Err(status
                        .reason
                        .unwrap_or_else(|| "Startup state did not change".into()));
                }
                commit_app_data(&state.0.data_path, &mut current, candidate)?;
                state
                    .0
                    .native_errors
                    .lock()
                    .map_err(lock_error)?
                    .autostart_error = None;
                Ok(status)
            },
            || restore(&app, &owner, snapshot),
        )
    })
    .await
}

fn with_rollback<T>(
    change: impl FnOnce() -> Result<T, String>,
    rollback: impl FnOnce() -> Result<(), String>,
) -> Result<T, String> {
    match change() {
        Ok(value) => Ok(value),
        Err(error) => match rollback() {
            Ok(()) => Err(error),
            Err(rollback) => Err(format!("{error}; startup rollback failed: {rollback}. Recheck the system startup settings.")),
        },
    }
}

#[cfg(any(target_os = "macos", test))]
fn launchagent_disabled(output: &str, label: &str) -> Result<bool, String> {
    let label = format!("\"{label}\"");
    for line in output.lines() {
        if let Some((key, value)) = line.split_once("=>") {
            if key.trim() == label {
                return match value.trim().trim_end_matches([',', ';']) {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err("Unknown LaunchAgent disabled state".into()),
                };
            }
        }
    }
    Ok(false)
}

fn validate_startup_path(owner: &AutostartOwner) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if Path::new(&owner.target).starts_with("/Volumes")
        || owner.target.contains("/AppTranslocation/")
    {
        return Err("temporaryPath".into());
    }
    let _ = owner;
    Ok(())
}

type Observed = (Option<AutostartOwner>, bool, Option<String>, bool);

#[cfg(target_os = "windows")]
fn platform_status(
    app: &AppHandle,
    current: &AutostartOwner,
    saved: Option<&AutostartOwner>,
) -> Result<Observed, String> {
    let target = windows::run_target(app)?;
    let native_owner = target
        .filter(|target| Path::new(target).is_file())
        .map(|target| path_owner(target, current, saved));
    if current.channel == "store" {
        let (enabled, reason) = windows::store_state()?;
        let conflict = native_owner.is_some();
        let owner = if enabled {
            Some(current.clone())
        } else {
            native_owner
        };
        return Ok((owner, enabled, reason, conflict));
    }
    let store_owner = saved.filter(|owner| owner.platform == "windows" && owner.channel == "store");
    let store_owner = match store_owner {
        Some(owner) if windows::package_installed(&owner.target)? => Some(owner.clone()),
        _ => None,
    };
    let enabled = native_owner
        .as_ref()
        .is_some_and(|owner| same_target(&owner.target, &current.target));
    Ok((store_owner.or(native_owner), enabled, None, false))
}

#[cfg(target_os = "macos")]
fn platform_status(
    app: &AppHandle,
    current: &AutostartOwner,
    saved: Option<&AutostartOwner>,
) -> Result<Observed, String> {
    let target = macos::target(app)?;
    let owner = target
        .filter(|target| Path::new(target).is_file())
        .map(|target| path_owner(target, current, saved));
    let disabled = macos::disabled(app)?;
    let enabled = !disabled
        && owner
            .as_ref()
            .is_some_and(|owner| same_target(&owner.target, &current.target));
    Ok((if disabled { None } else { owner }, enabled, None, false))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_status(
    app: &AppHandle,
    current: &AutostartOwner,
    _saved: Option<&AutostartOwner>,
) -> Result<Observed, String> {
    let enabled = app
        .autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())?;
    Ok((enabled.then(|| current.clone()), enabled, None, false))
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use ::windows::ApplicationModel::{StartupTask, StartupTaskState};
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE},
        RegKey, RegValue,
    };
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    pub fn package_family() -> Result<String, String> {
        use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;
        let mut length = 0;
        let status = unsafe { GetCurrentPackageFamilyName(&mut length, std::ptr::null_mut()) };
        if status != 122 {
            return Err(format!("Cannot read package identity: {status}"));
        }
        let mut buffer = vec![0; length as usize];
        let status = unsafe { GetCurrentPackageFamilyName(&mut length, buffer.as_mut_ptr()) };
        if status != 0 {
            return Err(format!("Cannot read package identity: {status}"));
        }
        String::from_utf16(&buffer[..length as usize - 1]).map_err(|error| error.to_string())
    }

    pub fn package_installed(family: &str) -> Result<bool, String> {
        use windows_sys::Win32::Storage::Packaging::Appx::GetPackagesByPackageFamily;
        let family: Vec<u16> = family.encode_utf16().chain(Some(0)).collect();
        let mut count = 0;
        let mut length = 0;
        let status = unsafe {
            GetPackagesByPackageFamily(
                family.as_ptr(),
                &mut count,
                std::ptr::null_mut(),
                &mut length,
                std::ptr::null_mut(),
            )
        };
        if status != 0 && status != 122 {
            return Err(format!("Cannot query installed package: {status}"));
        }
        if count == 0 {
            return Ok(false);
        }
        let mut names = vec![std::ptr::null_mut(); count as usize];
        let mut buffer = vec![0u16; length as usize];
        let status = unsafe {
            GetPackagesByPackageFamily(
                family.as_ptr(),
                &mut count,
                names.as_mut_ptr(),
                &mut length,
                buffer.as_mut_ptr(),
            )
        };
        if status != 0 {
            return Err(format!("Cannot query installed package: {status}"));
        }
        Ok(count != 0)
    }

    fn task() -> Result<StartupTask, String> {
        StartupTask::GetAsync(&"知歇Startup".into())
            .and_then(|operation| operation.get())
            .map_err(|error| error.to_string())
    }

    fn state_result(state: StartupTaskState) -> Result<(bool, Option<String>), String> {
        match state {
            StartupTaskState::Enabled => Ok((true, None)),
            StartupTaskState::EnabledByPolicy => Ok((true, Some("enabledByPolicy".into()))),
            StartupTaskState::Disabled => Ok((false, None)),
            StartupTaskState::DisabledByUser => Ok((false, Some("disabledByUser".into()))),
            StartupTaskState::DisabledByPolicy => Ok((false, Some("disabledByPolicy".into()))),
            _ => Err("Unknown Windows startup task state".into()),
        }
    }

    pub fn store_state() -> Result<(bool, Option<String>), String> {
        state_result(task()?.State().map_err(|error| error.to_string())?)
    }

    pub fn change_store(enabled: bool) -> Result<(), String> {
        let task = task()?;
        let state = task.State().map_err(|error| error.to_string())?;
        if state == StartupTaskState::DisabledByPolicy || state == StartupTaskState::EnabledByPolicy
        {
            return Err("disabledByPolicy".into());
        }
        if enabled {
            if state == StartupTaskState::DisabledByUser {
                return Err("disabledByUser".into());
            }
            let state = task
                .RequestEnableAsync()
                .and_then(|operation| operation.get())
                .map_err(|error| error.to_string())?;
            let (active, reason) = state_result(state)?;
            if !active {
                return Err(
                    reason.unwrap_or_else(|| "Windows did not enable the startup task".into())
                );
            }
        } else {
            task.Disable().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn read_value(key: &str, name: &str) -> Result<Option<RegValue>, String> {
        let result = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(key, KEY_READ)
            .and_then(|key| key.get_raw_value(name));
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn run_target(app: &AppHandle) -> Result<Option<String>, String> {
        if read_value(RUN, &app.package_info().name)?.is_none() {
            return Ok(None);
        }
        if !app
            .autolaunch()
            .is_enabled()
            .map_err(|error| error.to_string())?
        {
            return Ok(None);
        }
        let command: String = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN)
            .and_then(|key| key.get_value(&app.package_info().name))
            .map_err(|error| error.to_string())?;
        Ok(Some(command.trim().trim_matches('"').to_string()))
    }

    pub fn enable_run(app: &AppHandle, target: &str) -> Result<(), String> {
        app.autolaunch()
            .enable()
            .map_err(|error| error.to_string())?;
        // The plugin does not quote Windows paths; Program Files needs a quoted command.
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN, KEY_SET_VALUE)
            .and_then(|key| key.set_value(&app.package_info().name, &format!("\"{target}\"")))
            .map_err(|error| error.to_string())
    }

    pub type RegistrySnapshot = (Option<RegValue>, Option<RegValue>);
    pub fn registry_snapshot(app: &AppHandle) -> Result<RegistrySnapshot, String> {
        Ok((
            read_value(RUN, &app.package_info().name)?,
            read_value(APPROVED, &app.package_info().name)?,
        ))
    }

    pub fn restore_registry(app: &AppHandle, snapshot: RegistrySnapshot) -> Result<(), String> {
        for (path, value) in [(RUN, snapshot.0), (APPROVED, snapshot.1)] {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey_with_flags(path, KEY_SET_VALUE)
                .map_err(|error| error.to_string())?;
            match value {
                Some(value) => key
                    .set_raw_value(&app.package_info().name, &value)
                    .map_err(|error| error.to_string())?,
                None => {
                    if let Err(error) = key.delete_value(&app.package_info().name) {
                        if error.kind() != std::io::ErrorKind::NotFound {
                            return Err(error.to_string());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::process::Command;

    pub fn file(app: &AppHandle) -> Result<PathBuf, String> {
        app.path()
            .home_dir()
            .map(|home| {
                home.join("Library/LaunchAgents")
                    .join(format!("{}.plist", app.package_info().name))
            })
            .map_err(|error| error.to_string())
    }

    fn domain() -> Result<String, String> {
        let output = Command::new("/usr/bin/id")
            .arg("-u")
            .output()
            .map_err(|error| error.to_string())?;
        let uid = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
        if !output.status.success() || uid.trim().parse::<u32>().is_err() {
            return Err("Cannot determine the login user".into());
        }
        Ok(format!("gui/{}", uid.trim()))
    }

    pub fn disabled(app: &AppHandle) -> Result<bool, String> {
        let output = Command::new("/bin/launchctl")
            .args(["print-disabled", &domain()?])
            .output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        let output = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
        launchagent_disabled(&output, &app.package_info().name)
    }

    pub fn set_disabled(app: &AppHandle, disabled: bool) -> Result<(), String> {
        let target = format!("{}/{}", domain()?, app.package_info().name);
        let output = Command::new("/bin/launchctl")
            .args([if disabled { "disable" } else { "enable" }, &target])
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).into_owned())
        }
    }

    pub fn target(app: &AppHandle) -> Result<Option<String>, String> {
        let path = file(app)?;
        if !path.exists() {
            return Ok(None);
        }
        let plist = plist::Value::from_file(path).map_err(|error| error.to_string())?;
        let dict = plist.as_dictionary().ok_or("Invalid LaunchAgent plist")?;
        if dict.get("RunAtLoad").and_then(plist::Value::as_boolean) != Some(true) {
            return Ok(None);
        }
        let target = dict
            .get("ProgramArguments")
            .and_then(plist::Value::as_array)
            .and_then(|args| args.first())
            .and_then(plist::Value::as_string)
            .ok_or("Missing LaunchAgent executable")?;
        Ok(Some(target.into()))
    }

    pub fn change(app: &AppHandle, owner: &AutostartOwner, enabled: bool) -> Result<(), String> {
        let path = file(app)?;
        if enabled {
            std::fs::create_dir_all(path.parent().ok_or("Missing LaunchAgent directory")?)
                .map_err(|error| error.to_string())?;
            let mut dict = plist::Dictionary::new();
            dict.insert(
                "Label".into(),
                plist::Value::String(app.package_info().name.clone()),
            );
            dict.insert(
                "ProgramArguments".into(),
                plist::Value::Array(vec![plist::Value::String(owner.target.clone())]),
            );
            dict.insert("RunAtLoad".into(), plist::Value::Boolean(true));
            let mut bytes = Vec::new();
            plist::Value::Dictionary(dict)
                .to_writer_xml(&mut bytes)
                .map_err(|error| error.to_string())?;
            crate::atomic_write(&path, &bytes).map_err(|error| error.to_string())?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        set_disabled(app, !enabled)
    }
}

#[cfg(target_os = "windows")]
enum Snapshot {
    Store(bool),
    Registry(windows::RegistrySnapshot),
}
#[cfg(target_os = "macos")]
struct Snapshot {
    bytes: Option<Vec<u8>>,
    disabled: bool,
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
struct Snapshot(bool);

fn snapshot(app: &AppHandle, owner: &AutostartOwner) -> Result<Snapshot, String> {
    #[cfg(target_os = "windows")]
    return if owner.channel == "store" {
        Ok(Snapshot::Store(windows::store_state()?.0))
    } else {
        Ok(Snapshot::Registry(windows::registry_snapshot(app)?))
    };
    #[cfg(target_os = "macos")]
    {
        let _ = owner;
        let bytes = match std::fs::read(macos::file(app)?) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        Ok(Snapshot {
            bytes,
            disabled: macos::disabled(app)?,
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = owner;
        Ok(Snapshot(
            app.autolaunch()
                .is_enabled()
                .map_err(|error| error.to_string())?,
        ))
    }
}

fn change_native(app: &AppHandle, owner: &AutostartOwner, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if owner.channel == "store" {
        return windows::change_store(enabled);
    }
    #[cfg(target_os = "windows")]
    if enabled {
        return windows::enable_run(app, &owner.target);
    }
    #[cfg(target_os = "macos")]
    return macos::change(app, owner, enabled);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = owner;
        if enabled {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        }
        .map_err(|error| error.to_string())
    }
}

fn restore(app: &AppHandle, owner: &AutostartOwner, snapshot: Snapshot) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = owner;
        match snapshot {
            Snapshot::Store(enabled) => {
                if windows::store_state()?.0 == enabled {
                    Ok(())
                } else {
                    windows::change_store(enabled)
                }
            }
            Snapshot::Registry(snapshot) => windows::restore_registry(app, snapshot),
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = owner;
        let path = macos::file(app)?;
        match snapshot.bytes {
            Some(bytes) => crate::atomic_write(&path, &bytes).map_err(|error| error.to_string())?,
            None => {
                if path.exists() {
                    std::fs::remove_file(path).map_err(|error| error.to_string())?;
                }
            }
        }
        macos::set_disabled(app, snapshot.disabled)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    change_native(app, owner, snapshot.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_copies_can_replace_each_other_but_store_switches_conflict() {
        let owner = |channel: &str| AutostartOwner {
            platform: "windows".into(),
            channel: channel.into(),
            target: "test".into(),
        };
        assert!(!cross_channel(&owner("installed"), &owner("portable")));
        assert!(cross_channel(&owner("store"), &owner("portable")));
        assert!(cross_channel(&owner("installed"), &owner("store")));
    }

    #[test]
    fn saves_and_imports_keep_local_startup_state() {
        let mut local = AppData::default();
        local.settings.autostart = true;
        local.autostart_owner = Some(AutostartOwner {
            platform: "windows".into(),
            channel: "store".into(),
            target: "package".into(),
        });
        let mut imported = AppData::default();
        preserve_local_state(&mut imported, &local);
        assert!(imported.settings.autostart);
        assert_eq!(imported.autostart_owner, local.autostart_owner);
    }

    #[test]
    fn absent_owner_is_compatible_and_export_omits_it() {
        let data = AppData::default();
        let value = serde_json::to_value(&data).unwrap();
        assert!(value.get("autostartOwner").is_none());
        let read: AppData = serde_json::from_value(value).unwrap();
        assert_eq!(read.version, 6);
        assert!(read.autostart_owner.is_none());
        assert!(!read.settings.autostart);
    }

    #[test]
    fn failed_native_verification_or_save_restores_previous_registration() {
        use std::cell::Cell;
        for error in ["native verification failed", "settings disk full"] {
            let registration = Cell::new("old");
            let result: Result<(), String> = with_rollback(
                || {
                    registration.set("new");
                    Err(error.into())
                },
                || {
                    registration.set("old");
                    Ok(())
                },
            );
            assert_eq!(result.unwrap_err(), error);
            assert_eq!(registration.get(), "old");
        }
        let result: Result<(), String> = with_rollback(
            || Err("settings disk full".into()),
            || Err("access denied".into()),
        );
        let error = result.unwrap_err();
        assert!(error.contains("settings disk full"));
        assert!(error.contains("startup rollback failed: access denied"));
        let result = with_rollback(|| Ok(42), || panic!("successful change must not roll back"));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn launchctl_output_only_uses_the_zhixie_label() {
        assert!(
            launchagent_disabled("disabled services = {\n\"知歇\" => true\n}", "知歇")
                .unwrap()
        );
        assert!(
            !launchagent_disabled("\"OtherApp\" => true\n\"知歇\" => false", "知歇")
                .unwrap()
        );
        assert!(!launchagent_disabled("disabled services = {}", "知歇").unwrap());
        assert!(launchagent_disabled("\"知歇\" => unknown", "知歇").is_err());
    }
}
