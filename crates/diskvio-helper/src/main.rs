//! Non-privileged placeholder. Not bundled or launched by the desktop app.
fn main() {
    if std::env::args_os().len() > 1 {
        eprintln!("Diskvio helper: no operations or arguments are supported.");
        std::process::exit(2);
    }
    println!("Diskvio helper is inactive. No privileged or disk operations are implemented.");
}
