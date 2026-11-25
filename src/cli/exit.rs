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
    if let Some(hint) = typo_hint(&std::env::args_os().collect::<Vec<_>>(), &error.kind()) {
        eprintln!("hint: did you mean `{hint}`?");
    }
    REFUSAL
}

fn typo_hint(args: &[std::ffi::OsString], kind: &clap::error::ErrorKind) -> Option<&'static str> {
    let tokens = args
        .iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .collect::<Vec<_>>();
    let flags = [
        "--help",
        "--version",
        "--json",
        "--robot-triage",
        "--include",
        "--exclude",
        "--no-follow",
        "--no-witness",
        "--describe",
        "--schema",
        "--progress",
        "--fix",
    ];
    let top_level = ["capabilities", "robot-docs", "witness", "doctor"];
    let witness_actions = ["query", "last", "count"];
    let doctor_actions = ["health", "capabilities", "triage", "robot-docs"];
    let robot_docs_actions = ["guide"];

    for (index, token) in tokens.iter().enumerate() {
        let candidates: &[&str] = if token.starts_with('-') {
            &flags
        } else if tokens.first() == Some(&"witness") && index == 1 {
            &witness_actions
        } else if tokens.first() == Some(&"doctor") && index == 1 {
            &doctor_actions
        } else if tokens.first() == Some(&"robot-docs") && index == 1 {
            &robot_docs_actions
        } else if *kind == clap::error::ErrorKind::InvalidSubcommand && index == 0 {
            &top_level
        } else {
            continue;
        };

        let matches = candidates
            .iter()
            .copied()
            .filter(|candidate| edit_distance_at_most_one(token, candidate))
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return matches.first().copied();
        }
    }
    None
}

fn edit_distance_at_most_one(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left == right || left.len().abs_diff(right.len()) > 1 {
        return false;
    }

    if left.len() == right.len() {
        return left
            .iter()
            .zip(right)
            .filter(|(left, right)| left != right)
            .count()
            == 1;
    }

    let (shorter, longer) = if left.len() < right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let mut short_index = 0;
    let mut long_index = 0;
    let mut skipped = false;
    while short_index < shorter.len() {
        if shorter[short_index] == longer[long_index] {
            short_index += 1;
            long_index += 1;
        } else if skipped {
            return false;
        } else {
            skipped = true;
            long_index += 1;
        }
    }
    true
}
