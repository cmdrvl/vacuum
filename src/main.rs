#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(vacuum::run())
}
