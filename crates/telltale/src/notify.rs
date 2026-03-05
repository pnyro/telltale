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
        Box::new(windows_impl::WindowsToastNotifier::new("Telltale"))
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
    use windows::core::HSTRING;

    use telltale_core::Alert;

    use super::Notifier;

    pub struct WindowsToastNotifier {
        app_id: String,
    }

    impl WindowsToastNotifier {
        pub fn new(app_id: &str) -> Self {
            Self {
                app_id: app_id.to_string(),
            }
        }
    }

    impl Notifier for WindowsToastNotifier {
        fn notify(&self, alert: &Alert) -> Result<(), Box<dyn Error + Send + Sync>> {
            let title = alert.title.clone();
            let body = format!("{} Action: {}", alert.description, alert.recommended_action);
            let app_id = self.app_id.clone();

            thread::Builder::new()
                .name("telltale-toast".to_string())
                .spawn(move || {
                    if let Err(err) = show_toast(&app_id, &title, &body) {
                        eprintln!("notification error: {err}");
                    }
                })
                .map(|_| ())
                .map_err(|err| err.into())
        }
    }

    fn show_toast(app_id: &str, title: &str, body: &str) -> windows::core::Result<()> {
        let title = escape_xml(title);
        let body = escape_xml(body);
        let payload = format!(
            "<toast><visual><binding template=\"ToastGeneric\"><text>{title}</text><text>{body}</text></binding></visual></toast>"
        );

        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(payload))?;

        let toast = ToastNotification::CreateToastNotification(&doc)?;
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?;
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
