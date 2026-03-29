fn main() {
    if let Err(err) = bmx_rs::run() {
        eprintln!("bmx error: {err:#}");
        std::process::exit(1);
    }
}
