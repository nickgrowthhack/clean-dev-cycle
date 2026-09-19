use std::{env, fs};
fn main() {
    let args: Vec<_> = env::args().collect();
    if args.iter().any(|a| a == "POST" || a == "PATCH" || a == "DELETE") {
        eprintln!("mutação inesperada na preparação");
        std::process::exit(1);
    }
    let endpoint = args.last().unwrap();
    if endpoint.contains("/releases?") {
        let list = env::var_os("FAKE_GH_RELEASES").map(|p| fs::read_to_string(p).unwrap()).unwrap_or("[]".into());
        println!("{list}");
    } else if endpoint.contains("/git/ref/tags/") {
        println!("{{\"object\":{{\"type\":\"commit\",\"sha\":\"{}\"}}}}", env::var("FAKE_GH_SHA").unwrap());
    } else {
        eprintln!("requisição inesperada: {endpoint}");
        std::process::exit(1);
    }
}
