//! バージョン情報。

use std::sync::LazyLock;

#[rustfmt::skip] const GIT_BRANCH:    &str = env!("BUILD_GIT_BRANCH");
#[allow(dead_code)]
#[rustfmt::skip] const GIT_HASH:      &str = env!("BUILD_GIT_HASH");
#[rustfmt::skip] const GIT_DESCRIBE:  &str = env!("BUILD_GIT_DESCRIBE");
#[rustfmt::skip] const GIT_DATE:      &str = env!("BUILD_GIT_DATE");
#[rustfmt::skip] const BUILD_DEBUG:   &str = env!("BUILD_CARGO_DEBUG");
#[rustfmt::skip] const BUILD_TARGET:  &str = env!("BUILD_CARGO_TARGET");

/// rustc コンパイラバージョン "major.minor.patch"
pub fn rustc_version() -> &'static str {
    static RUSTC_VERSION: LazyLock<String> = LazyLock::new(|| {
        let meta = rustc_version_runtime::version_meta();
        format!("{} {:?}", meta.short_version_string, meta.channel)
    });

    &RUSTC_VERSION
}

/// ビルドプロファイルを "debug" または "release" で返す。
pub fn build_profile() -> &'static str {
    if BUILD_DEBUG == "true" {
        "debug"
    } else {
        "release"
    }
}

/// バージョン情報を読みやすい形の複数行文字列で返す。
#[rustfmt::skip]
pub fn version_info() -> &'static str {
    static VERSION_INFO: LazyLock<String> = LazyLock::new(||{
        let prof = build_profile();
        let rustc = rustc_version();

        format!(
"Build: {prof}
Branch: {GIT_BRANCH} {GIT_DESCRIBE} {GIT_DATE}
{rustc}"
    )
    });

    &VERSION_INFO
}

/// バージョン情報を文字列ベクタの形で返す。
pub fn version_info_vec() -> &'static Vec<String> {
    static VERSION_INFO_VEC: LazyLock<Vec<String>> = LazyLock::new(|| {
        let prof = build_profile();
        let rustc = rustc_version();
        vec![
            format!("Build: {BUILD_TARGET} {prof}"),
            format!("Branch: {GIT_BRANCH}"),
            format!("Version: {GIT_DESCRIBE}"),
            format!("Last Updated: {GIT_DATE}"),
            rustc.to_string(),
        ]
    });

    &VERSION_INFO_VEC
}
