use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn main() {
    let timestamp = OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("zero nanosecond should be valid")
        .format(&Rfc3339)
        .expect("UTC build timestamp should format as RFC3339");

    println!("cargo:rustc-env=VIDA_BUILD_TIMESTAMP_UTC={timestamp}");
}
