pub const SCAN_COMPLETE: u8 = 0;
pub const REFUSAL: u8 = 2;

pub fn from_clap_error(error: clap::Error) -> u8 {
    if matches!(
        error.kind(),
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
    ) {
        if let Err(_print_error) = error.print() {}
        return SCAN_COMPLETE;
    }

    match error.print() {
        Ok(()) | Err(_) => {}
    }
    REFUSAL
}
