//! Instance-wide settings of the tasks module, as the administrator left them in
//! the console.
//!
//! Declared by `module.toml`'s `[[settings]]`, stored in `core.settings`, and read
//! back here through `/internal/modules/tasks/settings` — a module owns its own
//! schema and cannot read the core's tables, and a background worker has no user
//! token for the public config route. The module is named in the URL so the read
//! works whether the instance shares one master secret or a derived one per
//! module.
//!
//! Every field here is read by code that acts on it: a knob that changes nothing
//! is worse than an absent one.

use serde_json::Value;

/// Number of bytes in one mebibyte, used to turn the admin's megabyte ceiling
/// into the byte count an attachment declares.
pub const BYTES_PER_MB: i64 = 1_048_576;

#[derive(Debug, Clone, Copy)]
pub struct InstanceConfig {
    /// Ceiling, in megabytes, on the declared size of a task attachment. `0` =
    /// no ceiling. Enforced when a new attachment row is recorded: the real file
    /// lives in drive/core (referenced by `file_id`), so this caps the size the
    /// task attachment declares at record time — the honest, sufficient point of
    /// application on the tasks side.
    pub attachment_max_mb: i64,
    /// Whether a board may be shared with other accounts at all.
    pub allow_board_sharing: bool,
    /// Ceiling on the number of boards one account may own. `0` = no ceiling.
    /// The automatic default board is never refused by it.
    pub max_boards_per_user: i64,
    /// Ceiling on the number of tasks a single board may hold. `0` = no ceiling.
    pub max_tasks_per_board: i64,
    /// Days a completed task is kept before the cleaner purges it. `0` = never.
    pub completed_task_retention_days: i64,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            attachment_max_mb:             25,
            allow_board_sharing:           true,
            max_boards_per_user:           0,
            max_tasks_per_board:           0,
            completed_task_retention_days: 0,
        }
    }
}

impl InstanceConfig {
    /// Maps the core's `{key: value}` object onto the struct. Every read falls
    /// back to the compiled default rather than to a permissive value; an
    /// out-of-range number is treated as a mistake and ignored the same way.
    /// `0` is a MEANINGFUL value for every ceiling here (no ceiling / never), so
    /// it is accepted rather than floored away.
    pub fn from_settings(settings: &Value) -> Self {
        let d = Self::default();
        let int_in = |key: &str, min: i64, max: i64, fallback: i64| -> i64 {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .filter(|n| (min..=max).contains(n))
                .unwrap_or(fallback)
        };
        let bool_of = |key: &str, fallback: bool| {
            settings.get(key).and_then(Value::as_bool).unwrap_or(fallback)
        };
        Self {
            attachment_max_mb: int_in("attachment_max_mb", 0, 1_048_576, d.attachment_max_mb),
            allow_board_sharing: bool_of("allow_board_sharing", d.allow_board_sharing),
            max_boards_per_user: int_in("max_boards_per_user", 0, 10_000, d.max_boards_per_user),
            max_tasks_per_board: int_in("max_tasks_per_board", 0, 1_000_000, d.max_tasks_per_board),
            completed_task_retention_days: int_in(
                "completed_task_retention_days", 0, 3650, d.completed_task_retention_days,
            ),
        }
    }
}

/// Reads the instance settings from the core. Any failure yields `None`, so the
/// caller keeps the values it already had rather than reverting to defaults
/// because the core was briefly unreachable.
pub async fn fetch(http: &reqwest::Client, core_url: &str, secret: &str) -> Option<InstanceConfig> {
    let url = format!("{core_url}/internal/modules/tasks/settings");
    let resp = http
        .get(&url)
        .header("X-Internal-Secret", secret)
        .send()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Lecture des réglages d'instance tasks"))
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "Réglages d'instance tasks refusés par le core");
        return None;
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Réglages d'instance tasks : réponse illisible"))
        .ok()?;

    Some(InstanceConfig::from_settings(body.get("settings")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_keys_keep_the_compiled_defaults() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert_eq!(c.attachment_max_mb, 25);
    }

    #[test]
    fn zero_means_no_ceiling() {
        let c = InstanceConfig::from_settings(&json!({ "attachment_max_mb": 0 }));
        assert_eq!(c.attachment_max_mb, 0);
    }

    #[test]
    fn negative_value_falls_back_to_default() {
        let c = InstanceConfig::from_settings(&json!({ "attachment_max_mb": -5 }));
        assert_eq!(c.attachment_max_mb, 25);
    }

    #[test]
    fn ceilings_and_switches_are_read() {
        let c = InstanceConfig::from_settings(&json!({
            "allow_board_sharing":           false,
            "max_boards_per_user":           12,
            "max_tasks_per_board":           500,
            "completed_task_retention_days": 90,
        }));
        assert!(!c.allow_board_sharing);
        assert_eq!(c.max_boards_per_user, 12);
        assert_eq!(c.max_tasks_per_board, 500);
        assert_eq!(c.completed_task_retention_days, 90);
    }

    #[test]
    fn out_of_range_ceiling_falls_back_to_default() {
        let c = InstanceConfig::from_settings(&json!({ "completed_task_retention_days": 5_000 }));
        assert_eq!(c.completed_task_retention_days, 0);
    }
}
