use std::error::Error;

use telltale_core::{Alert, Severity};

pub trait Notifier: Send + Sync {
    fn notify(&self, alert: &Alert) -> Result<(), Box<dyn Error + Send + Sync>>;
}

pub fn is_notifiable_severity(severity: Severity) -> bool {
    matches!(severity, Severity::Critical | Severity::Warning)
}

pub fn default_notifier() -> Box<dyn Notifier> {
    #[cfg(target_os = "windows")]
    {
        let notifier = windows_impl::WindowsToastNotifier::new();
        if let Err(err) = notifier.ensure_registered() {
            eprintln!("warning: failed to register app for notifications: {err}");
        }
        Box::new(notifier)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Box::new(NoopNotifier)
    }
}

#[cfg(not(target_os = "windows"))]
struct NoopNotifier;

#[cfg(not(target_os = "windows"))]
impl Notifier for NoopNotifier {
    fn notify(&self, _alert: &Alert) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::error::Error;
    use std::thread;

    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
        RegCreateKeyExW, RegSetValueExW,
    };
    use windows::core::HSTRING;

    use telltale_core::Alert;

    use super::Notifier;

    const AUMID: &str = "Telltale.SystemMonitor";
    const DISPLAY_NAME: &str = "Telltale";

    pub struct WindowsToastNotifier;

    impl WindowsToastNotifier {
        pub fn new() -> Self {
            Self
        }

        /// Register Telltale's AUMID in the Windows registry so toast notifications
        /// are accepted and displayed. Creates a Start Menu shortcut-like registry
        /// entry under HKCU\Software\Classes\AppUserModelId\{AUMID}.
        pub fn ensure_registered(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            unsafe {
                let subkey = format!("Software\\Classes\\AppUserModelId\\{AUMID}");
                let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
                let mut hkey = HKEY::default();
                let mut disposition = 0u32;

                let result = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    windows::core::PCWSTR(subkey_w.as_ptr()),
                    0,
                    None,
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    None,
                    &mut hkey,
                    Some(&mut disposition as *mut u32 as *mut _),
                );

                if result != ERROR_SUCCESS {
                    return Err(format!("RegCreateKeyExW failed: {result:?}").into());
                }

                // Set DisplayName value
                let value_name: Vec<u16> = "DisplayName"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let value_data: Vec<u16> = DISPLAY_NAME
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                let set_result = RegSetValueExW(
                    hkey,
                    windows::core::PCWSTR(value_name.as_ptr()),
                    0,
                    REG_SZ,
                    Some(std::slice::from_raw_parts(
                        value_data.as_ptr() as *const u8,
                        value_data.len() * 2,
                    )),
                );

                let _ = RegCloseKey(hkey);

                if set_result != ERROR_SUCCESS {
                    return Err(format!("RegSetValueExW failed: {set_result:?}").into());
                }
            }

            Ok(())
        }
    }

    impl Notifier for WindowsToastNotifier {
        fn notify(&self, alert: &Alert) -> Result<(), Box<dyn Error + Send + Sync>> {
            let title = alert.title.clone();
            let body = format!("{} Action: {}", alert.description, alert.recommended_action);

            thread::Builder::new()
                .name("telltale-toast".to_string())
                .spawn(move || {
                    if let Err(err) = show_toast(&title, &body) {
                        eprintln!("notification error: {err}");
                    }
                })
                .map(|_| ())
                .map_err(|err| err.into())
        }
    }

    fn show_toast(title: &str, body: &str) -> windows::core::Result<()> {
        let title = escape_xml(title);
        let body = escape_xml(body);
        let payload = format!(
            "<toast>\
                <visual>\
                    <binding template=\"ToastGeneric\">\
                        <text>{title}</text>\
                        <text>{body}</text>\
                    </binding>\
                </visual>\
                <audio src=\"ms-winsoundevent:Notification.Default\"/>\
            </toast>"
        );

        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(payload))?;

        let toast = ToastNotification::CreateToastNotification(&doc)?;
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?;
        notifier.Show(&toast)
    }

    fn escape_xml(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}
