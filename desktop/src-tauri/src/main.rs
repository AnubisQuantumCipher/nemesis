fn main() {
    if let Err(error) = nemesis_desktop::run() {
        eprintln!("NEMESIS Desktop failed to start: {error}");
        std::process::exit(1);
    }
}
