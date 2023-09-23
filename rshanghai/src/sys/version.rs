//! バージョン情報。
//!
//! vergen クレートによる。
//! [build.rs] を参照。

/*
https://docs.rs/vergen/latest/vergen/struct.EmitBuilder.html#method.emit

cargo:rustc-env=VERGEN_BUILD_DATE=2023-01-04
cargo:rustc-env=VERGEN_BUILD_TIMESTAMP=2023-01-04T15:38:11.097507114Z
cargo:rustc-env=VERGEN_CARGO_DEBUG=true
cargo:rustc-env=VERGEN_CARGO_FEATURES=build,git
cargo:rustc-env=VERGEN_CARGO_OPT_LEVEL=1
cargo:rustc-env=VERGEN_CARGO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
cargo:rustc-env=VERGEN_GIT_BRANCH=feature/version8
cargo:rustc-env=VERGEN_GIT_COMMIT_AUTHOR_EMAIL=your@email.com
cargo:rustc-env=VERGEN_GIT_COMMIT_AUTHOR_NAME=Yoda
cargo:rustc-env=VERGEN_GIT_COMMIT_COUNT=476
cargo:rustc-env=VERGEN_GIT_COMMIT_DATE=2023-01-03
cargo:rustc-env=VERGEN_GIT_COMMIT_MESSAGE=The best message
cargo:rustc-env=VERGEN_GIT_COMMIT_TIMESTAMP=2023-01-03T14:08:12.000000000-05:00
cargo:rustc-env=VERGEN_GIT_DESCRIBE=7.4.4-103-g53ae8a6
cargo:rustc-env=VERGEN_GIT_SHA=53ae8a69ab7917a2909af40f2e5d015f5b29ae28
cargo:rustc-env=VERGEN_RUSTC_CHANNEL=nightly
cargo:rustc-env=VERGEN_RUSTC_COMMIT_DATE=2023-01-03
cargo:rustc-env=VERGEN_RUSTC_COMMIT_HASH=c7572670a1302f5c7e245d069200e22da9df0316
cargo:rustc-env=VERGEN_RUSTC_HOST_TRIPLE=x86_64-unknown-linux-gnu
cargo:rustc-env=VERGEN_RUSTC_LLVM_VERSION=15.0
cargo:rustc-env=VERGEN_RUSTC_SEMVER=1.68.0-nightly
cargo:rustc-env=VERGEN_SYSINFO_NAME=Arch Linux
cargo:rustc-env=VERGEN_SYSINFO_OS_VERSION=Linux  Arch Linux
cargo:rustc-env=VERGEN_SYSINFO_USER=jozias
cargo:rustc-env=VERGEN_SYSINFO_TOTAL_MEMORY=31 GiB
cargo:rustc-env=VERGEN_SYSINFO_CPU_VENDOR=AuthenticAMD
cargo:rustc-env=VERGEN_SYSINFO_CPU_CORE_COUNT=8
cargo:rustc-env=VERGEN_SYSINFO_CPU_NAME=cpu0,cpu1,cpu2,cpu3,cpu4,cpu5,cpu6,cpu7
cargo:rustc-env=VERGEN_SYSINFO_CPU_BRAND=AMD Ryzen Threadripper 1900X 8-Core Processor
cargo:rustc-env=VERGEN_SYSINFO_CPU_FREQUENCY=3792
cargo:rerun-if-changed=.git/HEAD
cargo:rerun-if-changed=.git/refs/heads/feature/version8
cargo:rerun-if-changed=build.rs
cargo:rerun-if-env-changed=VERGEN_IDEMPOTENT
cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH
*/

use rustc_version_runtime;
use std::sync::OnceLock;

#[rustfmt::skip] pub const GIT_BRANCH:    &str = env!("VERGEN_GIT_BRANCH");
#[rustfmt::skip] pub const GIT_HASH:      &str = env!("VERGEN_GIT_SHA");
#[rustfmt::skip] pub const GIT_DESCRIBE:  &str = env!("VERGEN_GIT_DESCRIBE");
#[rustfmt::skip] pub const GIT_DATE:      &str = env!("VERGEN_GIT_COMMIT_DATE");
#[rustfmt::skip] pub const GIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
#[rustfmt::skip] pub const BUILD_DEBUG:   &str = env!("VERGEN_CARGO_DEBUG");
#[rustfmt::skip] pub const BUILD_TARGET:  &str = env!("VERGEN_CARGO_TARGET_TRIPLE");

// OnceLock / LazyLock が stable になったら LazyLock に書き換えたほうがよい

// rustc "major.minor.patch"
pub fn build_profile() -> &'static str {
    static BUILD_PROFILE: OnceLock<&str> = OnceLock::new();

    BUILD_PROFILE.get_or_init(|| {
        if BUILD_DEBUG == "true" {
            "debug"
        } else {
            "release"
        }
    })
}

// rustc "major.minor.patch"
pub fn rustc_version() -> &'static str {
    static RUSTC_VERSION: OnceLock<String> = OnceLock::new();

    RUSTC_VERSION.get_or_init(|| {
        let meta = rustc_version_runtime::version_meta();
        format!("{} {:?}", meta.short_version_string, meta.channel)
    })
}

#[rustfmt::skip]
pub fn version_info() -> &'static str {
    static VERSION_INFO: OnceLock<String> = OnceLock::new();

    VERSION_INFO.get_or_init(|| {
        let prof = build_profile();
        let rustc = rustc_version();
        format!(
"Build: {prof}
Branch: {GIT_BRANCH} {GIT_DESCRIBE} {GIT_DATE}
{rustc}"
    )
})
}

pub fn version_info_vec() -> &'static Vec<String> {
    static VERSION_INFO_VEC: OnceLock<Vec<String>> = OnceLock::new();

    VERSION_INFO_VEC.get_or_init(|| {
        let prof = build_profile();
        let rustc = rustc_version();
        vec![
            format!("Build: {BUILD_TARGET} {prof}"),
            format!("Branch: {GIT_BRANCH}"),
            format!("Version: {GIT_DESCRIBE}"),
            format!("Last Updated: {GIT_DATE}"),
            rustc.to_string(),
        ]
    })
}
