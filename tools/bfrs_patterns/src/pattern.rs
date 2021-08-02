use bfrs_common::BFCommand;

#[derive(Debug)]
pub enum Pattern {
    Instruction(BFCommand),
    Address { binding: String },
}
