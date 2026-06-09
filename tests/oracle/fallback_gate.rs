use libqalculate_rust::ffi::FallbackState;

use super::{DiffMismatch, MismatchField};

pub(super) fn fallback_state_label(
    state: Option<FallbackState>,
    has_session_settings: bool,
) -> String {
    state
        .map(|state| state.label().to_string())
        .unwrap_or_else(|| {
            if has_session_settings {
                "unsupported-session-settings".to_string()
            } else {
                FallbackState::CppFallbackEnabled.label().to_string()
            }
        })
}

pub(super) fn native_pass_fallback_mismatch(
    batch_name: &str,
    case_index: usize,
    expression: &str,
    parity_status: &str,
    fallback_state: &str,
    session_commands: &[String],
) -> Option<DiffMismatch> {
    if parity_status != "native-pass" || fallback_state == FallbackState::Native.label() {
        return None;
    }

    Some(DiffMismatch {
        batch_file: batch_name.to_string(),
        case_index,
        expression: expression.to_string(),
        field: MismatchField::FallbackState,
        cpp_value: FallbackState::Native.label().to_string(),
        rust_value: fallback_state.to_string(),
        deviation_id: None,
        normalization_policy: "exact-utf8".to_string(),
        fallback_state: fallback_state.to_string(),
        session_commands: session_commands.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_pass_check_fails_if_fallback_active() {
        assert!(native_pass_fallback_mismatch(
            "test.batch",
            0,
            "1 + 1",
            "native-pass",
            FallbackState::Native.label(),
            &[],
        )
        .is_none());

        let mismatch = native_pass_fallback_mismatch(
            "test.batch",
            0,
            "1 + 1",
            "native-pass",
            FallbackState::CppFallbackEnabled.label(),
            &[],
        )
        .expect("native-pass must reject C++ fallback output");

        assert_eq!(mismatch.field, MismatchField::FallbackState);
        assert_eq!(mismatch.cpp_value, FallbackState::Native.label());
        assert_eq!(
            mismatch.rust_value,
            FallbackState::CppFallbackEnabled.label()
        );
    }

    #[test]
    fn fallback_state_label_parses_cli_metadata() {
        assert_eq!(
            fallback_state_label(
                FallbackState::from_marker("[qalc-rs-metadata] fallback=native"),
                false,
            ),
            FallbackState::Native.label()
        );
        assert_eq!(
            fallback_state_label(None, false),
            FallbackState::CppFallbackEnabled.label()
        );
        assert_eq!(
            fallback_state_label(None, true),
            "unsupported-session-settings"
        );
    }
}
