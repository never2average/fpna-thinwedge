#![warn(argument_comment_mismatch)]

fn create_thinwedge_url(base_url: Option<String>) -> String {
    let _ = base_url;
    String::new()
}

fn main() {
    let _ = create_thinwedge_url(/*api_base*/ None);
}
