use crate::cli::args::{Command, WitnessAction};

pub fn dispatch(command: &Command) -> u8 {
    match command {
        Command::Witness { action } => dispatch_witness(action),
    }
}

fn dispatch_witness(action: &WitnessAction) -> u8 {
    match action {
        WitnessAction::Query { json, .. }
        | WitnessAction::Last { json }
        | WitnessAction::Count { json, .. } => {
            if *json {
                println!("[]");
            }
        }
    }

    crate::cli::exit::SCAN_COMPLETE
}
