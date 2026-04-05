#[path = "common/mod.rs"]
mod support;

#[test]
fn golden_runtime_snapshots() {
    for case_dir in support::fixture_dirs(&support::repo_tests_dir().join("golden")) {
        let source = support::load_source(&case_dir);
        support::assert_case(&case_dir, support::command_snapshot(&case_dir, &source, "run"));
    }
}
