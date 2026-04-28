#![no_main]

use std::io::Write;

use libfuzzer_sys::fuzz_target;
use tempfile::NamedTempFile;

fuzz_target!(|data: &[u8]| {
    if let Ok(mut file) = NamedTempFile::new() {
        let _ = file.write_all(data);
    }
});
