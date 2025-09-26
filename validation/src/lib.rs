#[cfg(test)]
mod fixtures;

mod block;
mod constants;
mod header;

pub use crate::block::{BlockValidator, ValidateBlockError};
pub use crate::constants::max_target;
pub use crate::header::{HeaderStore, HeaderValidator, ValidateHeaderError};

type BlockHeight = u32;
