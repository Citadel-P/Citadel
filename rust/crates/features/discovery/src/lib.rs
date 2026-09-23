#![forbid(unsafe_code)]
mod error;
pub mod lookup;
pub mod search;
pub use error::SearchError;
pub use lookup::LookupResourceType;
pub use lookup::{
    LookupCaller, LookupError, LookupQuery, LookupReader, LookupResourceInfo, LookupResult,
};
pub use search::{
    GlobalSearchGroup, GlobalSearchItem, GlobalSearchMatch, GlobalSearchParent, GlobalSearchQuery,
    GlobalSearchReader, GlobalSearchResults, GlobalSearchStatus, ValidatedSearch,
};
