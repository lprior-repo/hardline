#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Write;
use tempfile::NamedTempFile;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut file) = NamedTempFile::new() {
        let _ = file.write_all(data);
    }
});
