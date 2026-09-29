/// 尝试使用系统默认浏览器打开指定 URL。
/// - 使用 `open` crate 跨平台打开浏览器（macOS: `open`、Windows: `ShellExecuteW`、Linux: `xdg-open`）。
/// - 打开失败不会中断主流程，仅记录日志。
pub fn open_url_by_browser(url: &str) {
    tracing::debug!(url, "opening link in default browser");

    open::that(url).unwrap_or_else(|err| {
        tracing::debug!(url, %err, "failed to open default browser, please open the link manually");
    });
}
