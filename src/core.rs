//! The core compiler (pure, memoized salsa queries)

pub(crate) mod common;
pub(crate) mod db;
pub(crate) mod file_scanning;
pub(crate) mod lexical_analysis;
pub(crate) mod semantic_analysis;
pub(crate) mod syntactic_analysis;
