use vergen::{Config, vergen, TimestampKind};

fn main() {
    let mut config = Config::default();
    *config.git_mut().commit_timestamp_kind_mut() = TimestampKind::All;
    *config.git_mut().semver_dirty_mut() = Some("-dirty");

    vergen(config).unwrap();
}
