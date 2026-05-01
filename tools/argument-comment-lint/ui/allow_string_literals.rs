#![warn(uncommented_anonymous_literal_argument)]

fn describe(prefix: &str, suffix: &str) {
    let _ = (prefix, suffix);
}

fn main() {
    describe("thinwedge", r"https://api.thinwedge.com/v1");
}
