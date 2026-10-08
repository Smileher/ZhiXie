use windows_sys::Win32::Storage::Packaging::Appx::GetCurrentApplicationUserModelId;

pub fn show_packaged(title: &str, body: &str) -> Result<(), String> {
    let mut length = 0;
    let status = unsafe { GetCurrentApplicationUserModelId(&mut length, std::ptr::null_mut()) };
    if status != 122 || length == 0 {
        return Err(format!("Failed to read packaged app ID (Windows error {status})"));
    }

    let mut buffer = vec![0u16; length as usize];
    let status = unsafe { GetCurrentApplicationUserModelId(&mut length, buffer.as_mut_ptr()) };
    if status != 0 {
        return Err(format!("Failed to read packaged app ID (Windows error {status})"));
    }
    let end = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    let app_id = String::from_utf16(&buffer[..end])
        .map_err(|error| format!("Invalid packaged app ID: {error}"))?;

    let mut notification = notify_rust::Notification::new();
    notification.summary(title).body(body).auto_icon().app_id(&app_id);
    notification.show().map(|_| ()).map_err(|error| error.to_string())
}
