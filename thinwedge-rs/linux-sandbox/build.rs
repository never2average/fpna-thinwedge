fn main() {
    println!("cargo:rerun-if-env-changed=THINWEDGE_BWRAP_SHA256");
    println!("cargo:rerun-if-env-changed=THINWEDGE_ENABLE_VENDORED_BWRAP");
    if std::env::var_os("THINWEDGE_ENABLE_VENDORED_BWRAP").is_none() {
        return;
    }
}
