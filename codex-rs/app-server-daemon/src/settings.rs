use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tokio::fs;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DaemonSettings {
    pub(crate) remote_control_enabled: bool,
    /// Seconds to wait after SIGTERM before force-terminating (SIGKILL) a draining
    /// app-server. `null`/absent → 60 (default); `0` → unbounded (never auto-SIGKILL
    /// while the app-server is still draining turns).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) stop_grace_period_secs: Option<u64>,
    /// Total stop budget in seconds before the daemon gives up on a stopping
    /// app-server. `null`/absent → 70 (default); `0` → wait indefinitely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) stop_timeout_secs: Option<u64>,
    /// Whether the daemon runs the auto-update loop. `null`/absent → true (default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) auto_update_enabled: Option<bool>,
}

impl DaemonSettings {
    pub(crate) fn auto_update_enabled(&self) -> bool {
        self.auto_update_enabled.unwrap_or(true)
    }

    pub(crate) async fn load(path: &Path) -> Result<Self> {
        let contents = match fs::read_to_string(path).await {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to read daemon settings {}", path.display()));
            }
        };

        serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse daemon settings {}", path.display()))
    }

    pub(crate) async fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!(
                    "failed to create daemon settings directory {}",
                    parent.display()
                )
            })?;
        }

        let contents = serde_json::to_vec_pretty(self).context("failed to serialize settings")?;
        fs::write(path, contents)
            .await
            .with_context(|| format!("failed to write daemon settings {}", path.display()))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use pretty_assertions::assert_eq;

    use super::DaemonSettings;

    #[test]
    fn daemon_settings_use_camel_case_json() {
        assert_eq!(
            serde_json::to_string(&DaemonSettings {
                remote_control_enabled: true,
                ..Default::default()
            })
            .expect("serialize"),
            r#"{"remoteControlEnabled":true}"#
        );
    }

    #[test]
    fn daemon_settings_default_to_current_behavior_when_absent() {
        let settings = DaemonSettings::default();
        assert!(settings.auto_update_enabled());
        assert_eq!(settings.stop_grace_period_secs, None);
        assert_eq!(settings.stop_timeout_secs, None);
    }
}
