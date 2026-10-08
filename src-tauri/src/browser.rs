/// 所有窗口在文档加载前安装限制，缓存窗口重建后也会自动生效。
pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let builder = tauri::plugin::Builder::new("desktop-browser-policy")
        .js_init_script_on_all_frames(include_str!("browser.js"));

    // WebView2 的部分浏览器快捷键由原生层处理，不能只依赖 DOM 拦截。
    #[cfg(target_os = "windows")]
    let builder = builder.on_webview_ready(|webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
        use windows_core::Interface;

        if let Err(error) = webview.with_webview(|platform| {
            let result = (|| -> windows_core::Result<()> {
                // Tauri 将回调调度到 WebView 所属线程，COM 对象仅在此回调内使用。
                unsafe {
                    let settings = platform.controller().CoreWebView2()?.Settings()?;
                    // DOM policy blocks non-editable targets while preserving input editing menus.
                    settings.SetAreDefaultContextMenusEnabled(true)?;
                    settings
                        .cast::<ICoreWebView2Settings3>()?
                        .SetAreBrowserAcceleratorKeysEnabled(false)?;
                }
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("Failed to disable native browser actions: {error}");
            }
        }) {
            eprintln!("Failed to configure native browser actions: {error}");
        }
    });

    builder.build()
}
