use dgm::{parse_source, tokenize_source};
#[path = "common/mod.rs"]
mod support;

#[test]
fn lexer_conformance() {
    for case_dir in support::fixture_dirs(&support::repo_tests_dir().join("conformance").join("lexer")) {
        let source = support::load_source(&case_dir);
        let actual = support::with_snapshot_field(&source, "tokens", tokenize_source(&source).unwrap());
        support::assert_case(&case_dir, actual);
    }
}

#[test]
fn parser_conformance() {
    for case_dir in support::fixture_dirs(&support::repo_tests_dir().join("conformance").join("parser")) {
        let source = support::load_source(&case_dir);
        let actual = support::with_snapshot_field(&source, "ast", parse_source(&source).unwrap());
        support::assert_case(&case_dir, actual);
    }
}

#[test]
fn runtime_conformance() {
    for case_dir in support::fixture_dirs(&support::repo_tests_dir().join("conformance").join("runtime")) {
        let source = support::load_source(&case_dir);
        support::assert_case(&case_dir, support::command_snapshot(&case_dir, &source, "run"));
    }
}

#[test]
fn error_conformance() {
    for case_dir in support::fixture_dirs(&support::repo_tests_dir().join("conformance").join("errors")) {
        let source = support::load_source(&case_dir);
        support::assert_case(&case_dir, support::command_snapshot(&case_dir, &source, "run"));
    }
}
