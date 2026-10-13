//! E2E tests for wecom-cli.
//!
//! These are binary-process-level tests that spawn the `wecom` binary and
//! test main.rs behaviors: .env loading, config.json parsing, env overrides,
//! and process-level logging.
//!
//! All library-level behavior is tested in `crates/wecom/test-e2e/`.

#![allow(unused_imports)]

mod helpers;
use helpers::*;

// ── startup ─────────────────────────────────────────────────

mod startup {
    use super::*;
    mod version {
        use super::*;
        include!("cases/startup/001-version/test.rs");
    }
}

// ── config ──────────────────────────────────────────────────

mod config {
    use super::*;
    mod invalid_config_json {
        use super::*;
        include!("cases/config/004-invalid-config-json/test.rs");
    }
}

// ── auth ────────────────────────────────────────────────────

mod auth {
    use super::*;
    mod legacy_migration {
        use super::*;
        include!("cases/auth/001-legacy-migration/test.rs");
    }
    mod auth_hidden_skip_auth_check {
        use super::*;
        include!("cases/auth/002-auth-hidden-skip-auth-check/test.rs");
    }
}

// ── logging ─────────────────────────────────────────────────

mod logging {
    use super::*;
    mod stderr_log {
        use super::*;
        include!("cases/logging/001-stderr-log/test.rs");
    }
    mod log_file {
        use super::*;
        include!("cases/logging/002-log-file/test.rs");
    }
}

// ── json repair ─────────────────────────────────────────────

mod json_repair {
    use super::*;
    mod stderr_hint {
        use super::*;
        include!("cases/repair/001-json-repair-stderr/test.rs");
    }
}
// ── sandbox paths ───────────────────────────────────────────
//
// 双实例接线（main.rs）：WorkspaceFs 读 cwd+tmp 全域 / 写 cwd+tmp/requests，
// CLI 配置目录经 extra deny 屏蔽；PrivateFs 走 config_dir roots。

mod sandbox_paths {
    use super::*;
    mod external_output_confined_to_roots {
        use super::*;
        include!("cases/sandbox_paths/001-external-output-confined-to-roots/test.rs");
    }
    mod workspace_roots {
        use super::*;
        include!("cases/sandbox_paths/002-workspace-roots/test.rs");
    }
    mod symlink_escape {
        use super::*;
        include!("cases/sandbox_paths/003-symlink-escape/test.rs");
    }
    mod deny_credential_read {
        use super::*;
        include!("cases/sandbox_paths/004-deny-credential-read/test.rs");
    }
    mod hardlink_alias_threat_model {
        use super::*;
        include!("cases/sandbox_paths/005-hardlink-alias-threat-model/test.rs");
    }
    mod dangerous_chars {
        use super::*;
        include!("cases/sandbox_paths/006-dangerous-chars/test.rs");
    }
    mod dotdot_escape {
        use super::*;
        include!("cases/sandbox_paths/007-dotdot-escape/test.rs");
    }
    mod config_dir_under_cwd_denied {
        use super::*;
        include!("cases/sandbox_paths/008-config-dir-under-cwd-denied/test.rs");
    }
    mod output_reserved_rolled_back_on_first_page_failure {
        use super::*;
        include!(
            "cases/sandbox_paths/009-output-reserved-rolled-back-on-first-page-failure/test.rs"
        );
    }
    mod ndjson_retained_after_first_page {
        use super::*;
        include!("cases/sandbox_paths/010-ndjson-retained-after-first-page/test.rs");
    }
    mod binary_download_interrupted_leaves_nothing {
        use super::*;
        include!("cases/sandbox_paths/011-binary-download-interrupted-leaves-nothing/test.rs");
    }
    mod binary_download_complete_retained {
        use super::*;
        include!("cases/sandbox_paths/012-binary-download-complete-retained/test.rs");
    }
}
