mod extensions;
mod support;

use crate::engine::TrainingError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionResult {
    pub name: &'static str,
    pub property: &'static str,
    pub passed: bool,
    pub detail: String,
}

pub fn run_extensions() -> Result<Vec<ExtensionResult>, TrainingError> {
    extensions::run()
}
