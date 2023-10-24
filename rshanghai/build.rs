use vergen::EmitBuilder;

fn main() {
    EmitBuilder::builder()
        .all_cargo()
        .all_git()
        .git_describe(true, false, None)
        .emit()
        .unwrap();
}
