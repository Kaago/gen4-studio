//! Structured error of the Tauri commands, so the UI can react to specific failures
//! (a file changed on disk, a missing tool) instead of parsing message text.

use g4_kernel::error::KernelError;
use g4_kernel::project::AssetId;
use serde::Serialize;

/// Error returned to the frontend. `code` is `"stale"`, `"tool_missing"` or `"error"`;
/// `asset` is set for `"stale"` and serializes as `null` otherwise.
#[derive(Debug, Clone, Serialize)]
pub struct CmdError {
    pub code: &'static str,
    pub message: String,
    pub asset: Option<AssetId>,
}

impl From<KernelError> for CmdError {
    fn from(e: KernelError) -> Self {
        let (code, asset) = match &e {
            KernelError::Stale(id) => ("stale", Some(*id)),
            KernelError::ToolMissing { .. } => ("tool_missing", None),
            _ => ("error", None),
        };
        CmdError {
            code,
            message: e.to_string(),
            asset,
        }
    }
}

impl From<String> for CmdError {
    fn from(message: String) -> Self {
        CmdError {
            code: "error",
            message,
            asset: None,
        }
    }
}

impl From<&str> for CmdError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use g4_kernel::project::AssetKind;

    #[test]
    fn kernel_errors_map_to_codes() {
        let id = AssetId {
            kind: AssetKind::Map,
            index: 0,
        };
        let stale = CmdError::from(KernelError::Stale(id));
        assert_eq!(stale.code, "stale");
        assert_eq!(stale.asset, Some(id));
        assert_eq!(
            stale.message,
            "map 0000 changed on disk since it was loaded"
        );

        let missing = CmdError::from(KernelError::ToolMissing {
            name: "apicula",
            reason: "not found".into(),
        });
        assert_eq!(missing.code, "tool_missing");
        assert!(missing.asset.is_none());
        assert!(missing.message.contains("apicula"));

        let other = CmdError::from(KernelError::NotEditable(AssetKind::Map));
        assert_eq!(other.code, "error");
        assert!(other.asset.is_none());
    }

    #[test]
    fn serializes_without_asset_as_null() {
        let json = serde_json::to_value(CmdError::from("boom".to_string())).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "error", "message": "boom", "asset": null })
        );
        let id = AssetId {
            kind: AssetKind::Map,
            index: 3,
        };
        let json = serde_json::to_value(CmdError::from(KernelError::Stale(id))).unwrap();
        assert_eq!(
            json["asset"],
            serde_json::json!({ "kind": "Map", "index": 3 })
        );
    }
}
