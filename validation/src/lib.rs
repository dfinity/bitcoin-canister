mod constants;
mod header;

pub use crate::header::{validate_header, HeaderStore, ValidateHeaderError};
pub use ic_btc_types;

type BlockHeight = u32;
