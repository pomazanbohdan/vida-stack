pub fn assert_shadow_report_clean(report: &serde_json::Value) {
    assert_eq!(report["unexplained_difference_count"], 0);
    assert_eq!(report["authoritative_write_count"], 0);
    assert_eq!(report["external_effect_count"], 0);
    assert_eq!(report["parity_gate"], "pass");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_shadow_report_assertion_accepts_zero_diff_no_write_report() {
        let report = serde_json::json!({
            "unexplained_difference_count": 0,
            "authoritative_write_count": 0,
            "external_effect_count": 0,
            "parity_gate": "pass"
        });

        assert_shadow_report_clean(&report);
    }

    #[test]
    fn clean_shadow_report_assertion_rejects_each_non_clean_gate_field() {
        let fields = [
            ("unexplained_difference_count", serde_json::json!(1)),
            ("authoritative_write_count", serde_json::json!(1)),
            ("external_effect_count", serde_json::json!(1)),
            ("parity_gate", serde_json::json!("blocked")),
        ];

        for (field, value) in fields {
            let mut report = serde_json::json!({
                "unexplained_difference_count": 0,
                "authoritative_write_count": 0,
                "external_effect_count": 0,
                "parity_gate": "pass"
            });
            report[field] = value;
            assert!(
                std::panic::catch_unwind(|| assert_shadow_report_clean(&report)).is_err(),
                "non-clean shadow field `{field}` must fail closed"
            );
        }
    }
}
