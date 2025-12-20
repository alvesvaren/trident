mod ast;
mod compile;
mod codegen;
mod update;
mod types;

pub use ast::parse_file;
pub use compile::compile;
pub use codegen::emit_file;
pub use update::{update_class_position, update_group_position, remove_class_position, remove_all_positions};
pub use types::*;
pub use compile::{Diagram, GroupId, ClassId, Group, Class, Edge};