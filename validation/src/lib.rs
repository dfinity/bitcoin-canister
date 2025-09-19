mod block;
mod constants;
mod header;

pub use crate::constants::max_target;
pub use crate::header::{validate_header, HeaderStore, HeaderValidator, ValidateHeaderError};

type BlockHeight = u32;
