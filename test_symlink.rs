use testscript_rs::testscript;

#[test]
fn test_symlink() {
    testscript::run("/tmp")
        .filter_file("test_symlink.txtar")
        .execute()
        .unwrap();
}
