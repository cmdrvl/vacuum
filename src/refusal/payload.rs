use crate::refusal::codes::RefusalCode;

pub fn print_missing_roots() {
    println!(
        "{{\"version\":\"vacuum.v0\",\"outcome\":\"REFUSAL\",\"refusal\":{{\"code\":\"{}\",\"message\":\"At least one root path is required\",\"detail\":{{}},\"next_command\":null}}}}",
        RefusalCode::RootNotFound.as_str()
    );
}
