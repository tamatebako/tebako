//! tebako#989's class-L fixture dylib source — never a cargo target.
//!
//! tests/interpose_dlopen.rs and ci/macos-signed-dlopen.sh both compile
//! this one file into `libdlprobe_fixture.dylib`:
//!
//! ```sh
//! rustc --edition 2021 --crate-type cdylib dlprobe_fixture.rs \
//!     -o libdlprobe_fixture.dylib
//! ```
//!
//! and stage it into the fixture env image at
//! `lib/dlprobe/libdlprobe_fixture.dylib` (the probe dlopens it as
//! `/__tfs__/lib/dlprobe/libdlprobe_fixture.dylib` through the interposed
//! route). The return value is the probe's expected answer — it and
//! `EXPECTED_ANSWER` in src/bin/tebako-dlprobe.rs move together or not
//! at all.

#[no_mangle]
pub extern "C" fn dlprobe_answer() -> u64 {
    0xD10C_CAFE_0989
}
