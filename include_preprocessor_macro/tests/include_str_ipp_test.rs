use include_preprocessor_macro::include_str_ipp;

#[test]
fn test_include_str_ipp() {
    let actual = include_str_ipp!("valid/a.txt");
    let expected = include_str!("expected.txt");

    assert_eq!(actual, expected);
}
