//! バージョン情報。
//!
//! vergen クレートによる。
//! [build.rs] を参照。

/*
https://docs.rs/vergen/latest/vergen/

Variable	Sample
See Build to configure the following
VERGEN_BUILD_DATE	2021-02-25
VERGEN_BUILD_TIME	23:28:39.493201
VERGEN_BUILD_TIMESTAMP	2021-02-25T23:28:39.493201+00:00
VERGEN_BUILD_SEMVER	5.0.0
See Git to configure the following
VERGEN_GIT_BRANCH	feature/fun
VERGEN_GIT_COMMIT_DATE	2021-02-24
VERGEN_GIT_COMMIT_TIME	20:55:21
VERGEN_GIT_COMMIT_TIMESTAMP	2021-02-24T20:55:21+00:00
VERGEN_GIT_SEMVER	5.0.0-2-gf49246c
VERGEN_GIT_SEMVER_LIGHTWEIGHT	feature-test
VERGEN_GIT_SHA	f49246ce334567bff9f950bfd0f3078184a2738a
VERGEN_GIT_SHA_SHORT	f49246c
See Rustc to configure the following
VERGEN_RUSTC_CHANNEL	nightly
VERGEN_RUSTC_COMMIT_DATE	2021-02-24
VERGEN_RUSTC_COMMIT_HASH	a8486b64b0c87dabd045453b6c81500015d122d6
VERGEN_RUSTC_HOST_TRIPLE	x86_64-apple-darwin
VERGEN_RUSTC_LLVM_VERSION	11.0
VERGEN_RUSTC_SEMVER	1.52.0-nightly
See Cargo to configure the following
VERGEN_CARGO_FEATURES	git,build
VERGEN_CARGO_PROFILE	debug
VERGEN_CARGO_TARGET_TRIPLE	x86_64-unknown-linux-gnu
See Sysinfo to configure the following
VERGEN_SYSINFO_NAME	Manjaro Linux
VERGEN_SYSINFO_OS_VERSION	Linux Manjaro Linux
VERGEN_SYSINFO_USER	Yoda
VERGEN_SYSINFO_TOTAL_MEMORY	33 GB
VERGEN_SYSINFO_CPU_VENDOR	Authentic AMD
VERGEN_SYSINFO_CPU_CORE_COUNT	8
VERGEN_SYSINFO_CPU_NAME	cpu0,cpu1,cpu2,cpu3,cpu4,cpu5,cpu6,cpu7
VERGEN_SYSINFO_CPU_BRAND	AMD Ryzen Threadripper 1900X 8-Core Processor
VERGEN_SYSINFO_CPU_FREQUENCY	3792
*/

use once_cell::sync::Lazy;

#[rustfmt::skip] pub const GIT_BRANCH:    &str = env!("VERGEN_GIT_BRANCH");
#[rustfmt::skip] pub const GIT_HASH:      &str = env!("VERGEN_GIT_SHA");
#[rustfmt::skip] pub const GIT_SEMVER:    &str = env!("VERGEN_GIT_SEMVER");
#[rustfmt::skip] pub const GIT_DATE:      &str = env!("VERGEN_GIT_COMMIT_DATE");
#[rustfmt::skip] pub const GIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");
#[rustfmt::skip] pub const BUILD_PROFILE: &str = env!("VERGEN_CARGO_PROFILE");
#[rustfmt::skip] pub const BUILD_TARGET:  &str = env!("VERGEN_CARGO_TARGET_TRIPLE");

#[rustfmt::skip]
pub static VERSION_INFO: Lazy<String> = Lazy::new(|| {
    format!(
"Build: {} {}
Branch: {}
{}
{}",
        BUILD_TARGET, BUILD_PROFILE, GIT_BRANCH, GIT_SEMVER, GIT_DATE
    )
});
