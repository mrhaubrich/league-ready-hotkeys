#[cfg(windows)]
fn main() {
    println!("league-ready-hotkeys foundation; Windows integration is under construction");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("league-ready-hotkeys is supported on Windows only");
}
