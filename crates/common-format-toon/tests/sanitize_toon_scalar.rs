use common_format_toon::sanitize_toon_scalar;

#[test]
fn safe_scalar_reserves_input_capacity() {
    let value = "plain";

    let sanitized = sanitize_toon_scalar(value);

    assert_eq!(sanitized, value);
    assert_eq!(sanitized.capacity(), value.len());
}
