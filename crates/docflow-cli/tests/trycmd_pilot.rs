#[test]
fn docflow_trycmd_pilot() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
