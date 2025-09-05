//! The process model for project1.
//!
//! This file defines the process model of the project1.

use crate::file_struct::FileStruct;

/// A process state of project 1, which contains file state.
#[derive(Default)]
pub struct Process {
    pub file_struct: FileStruct,
}
