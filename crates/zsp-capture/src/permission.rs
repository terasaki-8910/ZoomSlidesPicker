//! Screen Recording permission (macOS). Other platforms need none.

/// macOS: `CGPreflightScreenCaptureAccess()`. Other platforms: true.
pub fn screen_capture_granted() -> bool {
    todo!()
}

/// macOS: `CGRequestScreenCaptureAccess()` (shows the system prompt once; the app must be
/// restarted after the user allows it). Returns whether access is granted now.
/// Other platforms: true.
pub fn request_screen_capture() -> bool {
    todo!()
}

/// macOS: the System Settings page for Screen Recording,
/// `x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture`.
/// Other platforms: None.
pub fn settings_url() -> Option<&'static str> {
    todo!()
}
