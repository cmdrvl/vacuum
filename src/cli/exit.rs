pub const SCAN_COMPLETE: u8 = 0;
pub const REFUSAL: u8 = 2;

pub fn from_clap_error(error: clap::Error) -> u8 {
    match error.print() {
        Ok(()) | Err(_) => {}
    }
    REFUSAL
}
