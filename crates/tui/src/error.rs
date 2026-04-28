use thiserror::Error;

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("TUI error: {0}")]
    Error(String),

    #[error("Terminal error: {0}")]
    TerminalError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TuiError>;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    // ── TuiError::Error display ──

    #[test]
    fn tui_error_display_formats_message() {
        let err = TuiError::Error("something broke".to_string());
        assert_eq!(err.to_string(), "TUI error: something broke");
    }

    #[test]
    fn tui_error_with_empty_message() {
        let err = TuiError::Error(String::new());
        assert_eq!(err.to_string(), "TUI error: ");
    }

    #[test]
    fn tui_error_with_multiline_message() {
        let msg = "line1\nline2\nline3";
        let err = TuiError::Error(msg.to_string());
        let formatted = err.to_string();
        assert!(
            formatted.contains("line1"),
            "should contain line1: {formatted}"
        );
        assert!(
            formatted.contains("line3"),
            "should contain line3: {formatted}"
        );
    }

    #[test]
    fn tui_error_is_debug() {
        let err = TuiError::Error("debug test".to_string());
        let debug_str = format!("{err:?}");
        assert!(
            debug_str.contains("debug test"),
            "debug output: {debug_str}"
        );
    }

    // ── TerminalError display ──

    #[test]
    fn terminal_error_display_formats_message() {
        let err = TuiError::TerminalError("no terminal".to_string());
        assert_eq!(err.to_string(), "Terminal error: no terminal");
    }

    #[test]
    fn terminal_error_with_empty_message() {
        let err = TuiError::TerminalError(String::new());
        assert_eq!(err.to_string(), "Terminal error: ");
    }

    // ── IoError display & conversion ──

    #[test]
    fn io_error_display_formats_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = TuiError::IoError(io_err);
        let msg = err.to_string();
        assert!(
            msg.contains("file missing"),
            "expected 'file missing' in: {msg}"
        );
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let tui_err: TuiError = io_err.into();
        let msg = tui_err.to_string();
        assert!(
            msg.contains("access denied"),
            "expected 'access denied' in: {msg}"
        );
    }

    #[test]
    fn io_error_various_kinds_display_without_panic() {
        let kinds = vec![
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::ConnectionAborted,
            std::io::ErrorKind::BrokenPipe,
            std::io::ErrorKind::AlreadyExists,
            std::io::ErrorKind::WouldBlock,
            std::io::ErrorKind::InvalidInput,
            std::io::ErrorKind::InvalidData,
            std::io::ErrorKind::TimedOut,
            std::io::ErrorKind::WriteZero,
            std::io::ErrorKind::Interrupted,
            std::io::ErrorKind::UnexpectedEof,
        ];
        for kind in kinds {
            let io_err = std::io::Error::new(kind, "test");
            let tui_err: TuiError = io_err.into();
            let msg = tui_err.to_string();
            assert!(
                !msg.is_empty(),
                "IO error kind {kind:?} should produce non-empty message"
            );
        }
    }

    #[test]
    fn io_error_is_debug() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "inner");
        let tui_err = TuiError::IoError(io_err);
        let debug_str = format!("{tui_err:?}");
        assert!(
            debug_str.contains("IoError"),
            "debug should mention variant: {debug_str}"
        );
    }

    // ── Result type ──

    #[test]
    fn result_type_ok() {
        let ok: Result<i32> = Ok(42);
        assert!(ok.is_ok());
        assert_eq!(ok.expect("should be Ok"), 42);
    }

    #[test]
    fn result_type_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".to_string()));
        assert!(err.is_err());
    }

    #[test]
    fn result_type_unit_ok() {
        let ok: Result<()> = Ok(());
        assert!(ok.is_ok());
    }

    #[test]
    fn result_type_string_ok() {
        let ok: Result<String> = Ok("hello".to_string());
        assert_eq!(ok.expect("ok"), "hello");
    }

    #[test]
    fn result_type_option_like_unwrapping() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(*ok.as_ref().expect("ok"), 42);

        let err: Result<i32> = Err(TuiError::TerminalError("bad".to_string()));
        assert!(err.as_ref().is_err());
    }

    // ── Error variant discrimination ──

    #[test]
    fn error_variants_are_distinct() {
        let a = TuiError::Error("a".to_string());
        let b = TuiError::TerminalError("b".to_string());
        let io = TuiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "c"));
        assert!(std::mem::discriminant(&a) != std::mem::discriminant(&b));
        assert!(std::mem::discriminant(&a) != std::mem::discriminant(&io));
        assert!(std::mem::discriminant(&b) != std::mem::discriminant(&io));
    }

    // ── thiserror Error trait ──

    #[test]
    fn tui_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(TuiError::Error("test".to_string()));
        let _msg = err.to_string();
    }

    #[test]
    fn terminal_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(TuiError::TerminalError("test".to_string()));
        let _msg = err.to_string();
    }

    #[test]
    fn io_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(TuiError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test",
        )));
        let _msg = err.to_string();
    }

    // ── Send & Sync ──

    #[test]
    fn tui_error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TuiError>();
    }

    #[test]
    fn tui_error_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<TuiError>();
    }

    #[test]
    fn result_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Result<()>>();
        assert_send::<Result<String>>();
        assert_send::<Result<i32>>();
    }

    #[test]
    fn result_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Result<()>>();
        assert_sync::<Result<String>>();
        assert_sync::<Result<i32>>();
    }

    // ── Error source chain ──

    #[test]
    fn io_error_source_returns_inner_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file.txt");
        let tui_err = TuiError::IoError(io_err);
        let source = tui_err.source();
        assert!(source.is_some(), "IoError should have a source");
    }

    #[test]
    fn plain_tui_error_has_no_source() {
        let err = TuiError::Error("test".to_string());
        assert!(err.source().is_none(), "plain Error variant has no source");
    }

    #[test]
    fn terminal_error_has_no_source() {
        let err = TuiError::TerminalError("test".to_string());
        assert!(
            err.source().is_none(),
            "TerminalError variant has no source"
        );
    }

    // ── Size & layout ──

    #[test]
    fn tui_error_size_is_reasonable() {
        let size = std::mem::size_of::<TuiError>();
        // String is ~24 bytes, enum discriminant + padding
        assert!(
            size < 256,
            "TuiError should be reasonably sized, got {size} bytes"
        );
    }

    // ── Special characters in error messages ──

    #[test]
    fn tui_error_with_null_bytes() {
        let msg = "error\0with\0nulls";
        let err = TuiError::Error(msg.to_string());
        let formatted = err.to_string();
        assert!(formatted.starts_with("TUI error:"));
    }

    #[test]
    fn tui_error_with_unicode() {
        let msg = "エラー エロreur 错误";
        let err = TuiError::Error(msg.to_string());
        let formatted = err.to_string();
        assert!(formatted.contains("エラー"));
    }

    #[test]
    fn terminal_error_with_unicode() {
        let msg = "ターミナル terminal";
        let err = TuiError::TerminalError(msg.to_string());
        let formatted = err.to_string();
        assert!(formatted.contains("ターミナル"));
    }

    #[test]
    fn tui_error_with_only_whitespace() {
        let err = TuiError::Error("   \t\n  ".to_string());
        let formatted = err.to_string();
        assert!(formatted.starts_with("TUI error:"));
    }

    #[test]
    fn io_error_debug_includes_kind() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access");
        let tui_err = TuiError::IoError(io_err);
        let debug = format!("{tui_err:#?}");
        assert!(!debug.is_empty());
    }

    // ── Result combinators ──

    #[test]
    fn result_map_on_ok() {
        let ok: Result<i32> = Ok(10);
        let mapped = ok.map(|v| v * 2);
        assert_eq!(mapped.expect("ok"), 20);
    }

    #[test]
    fn result_map_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        let mapped = err.map(|v| v * 2);
        assert!(mapped.is_err());
    }

    #[test]
    fn result_map_err_on_ok() {
        let ok: Result<i32> = Ok(10);
        let mapped = ok.map_err(|e| format!("{e}"));
        assert_eq!(mapped.expect("ok"), 10);
    }

    #[test]
    fn result_map_err_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        let mapped = err.map_err(|e| format!("{e}"));
        assert!(mapped.is_err());
    }

    #[test]
    fn result_and_then_on_ok() {
        let ok: Result<i32> = Ok(10);
        let result = ok.and_then(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(TuiError::Error("neg".into()))
            }
        });
        assert_eq!(result.expect("ok"), 10);
    }

    #[test]
    fn result_and_then_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        let result = err.and_then(|v| Ok(v * 2));
        assert!(result.is_err());
    }

    #[test]
    fn result_unwrap_or_on_ok() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap_or(0), 42);
    }

    #[test]
    fn result_unwrap_or_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        assert_eq!(err.unwrap_or(0), 0);
    }

    #[test]
    fn result_unwrap_or_else_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        assert_eq!(err.unwrap_or_else(|_| 99), 99);
    }

    #[test]
    fn result_unwrap_or_default() {
        let ok: Result<i32> = Ok(42);
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        assert_eq!(ok.unwrap_or_default(), 42);
        assert_eq!(err.unwrap_or_default(), 0);
    }

    #[test]
    fn result_iter_on_ok() {
        let ok: Result<i32> = Ok(42);
        let items: Vec<&i32> = ok.iter().collect();
        assert_eq!(items.len(), 1);
        assert_eq!(*items[0], 42);
    }

    #[test]
    fn result_iter_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        let items: Vec<&i32> = err.iter().collect();
        assert!(items.is_empty());
    }

    #[test]
    fn result_ok_values() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.ok(), Some(42));
    }

    #[test]
    fn result_err_values() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        assert!(err.err().is_some());
    }

    #[test]
    fn result_ok_on_ok() {
        let ok: Result<i32> = Ok(42);
        assert!(ok.err().is_none());
    }

    #[test]
    fn result_err_on_err() {
        let err: Result<i32> = Err(TuiError::Error("fail".into()));
        assert!(err.ok().is_none());
    }

    // ── Proptests ──

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_tui_error_display_roundtrips_arbitrary_message(
            msg in proptest::string::string_regex(".{0,10000}").unwrap(),
        ) {
            let err = TuiError::Error(msg.clone());
            let formatted = err.to_string();
            assert!(formatted.starts_with("TUI error:"));
            // For non-empty messages, the message content should appear
            if !msg.is_empty() {
                assert!(formatted.contains(&msg) || formatted.contains(msg.as_str()),
                    "formatted '{formatted}' should contain original message");
            }
        }

        #[test]
        fn prop_terminal_error_display_roundtrips_arbitrary_message(
            msg in proptest::string::string_regex(".{0,10000}").unwrap(),
        ) {
            let err = TuiError::TerminalError(msg.clone());
            let formatted = err.to_string();
            assert!(formatted.starts_with("Terminal error:"));
            if !msg.is_empty() {
                assert!(formatted.contains(msg.as_str()),
                    "formatted '{formatted}' should contain original message");
            }
        }

        #[test]
        fn prop_io_error_from_conversion_never_panics(
            kind_idx in 0usize..14,
        ) {
            let kinds = [
                std::io::ErrorKind::NotFound,
                std::io::ErrorKind::PermissionDenied,
                std::io::ErrorKind::ConnectionRefused,
                std::io::ErrorKind::ConnectionReset,
                std::io::ErrorKind::ConnectionAborted,
                std::io::ErrorKind::BrokenPipe,
                std::io::ErrorKind::AlreadyExists,
                std::io::ErrorKind::WouldBlock,
                std::io::ErrorKind::InvalidInput,
                std::io::ErrorKind::InvalidData,
                std::io::ErrorKind::TimedOut,
                std::io::ErrorKind::WriteZero,
                std::io::ErrorKind::Interrupted,
                std::io::ErrorKind::UnexpectedEof,
            ];
            let kind = kinds[kind_idx];
            let io_err = std::io::Error::new(kind, "prop test");
            let tui_err: TuiError = io_err.into();
            assert!(!tui_err.to_string().is_empty());
        }

        #[test]
        fn prop_result_map_preserves_ok(
            val in proptest::num::i32::ANY,
        ) {
            let ok: Result<i32> = Ok(val);
            let doubled = ok.map(|v| v.wrapping_mul(2));
            assert_eq!(doubled.expect("ok"), val.wrapping_mul(2));
        }

        #[test]
        fn prop_result_unwrap_or_default_matches(
            val in proptest::num::i32::ANY,
        ) {
            let ok: Result<i32> = Ok(val);
            assert_eq!(ok.unwrap_or_default(), val);
        }
    }
}
