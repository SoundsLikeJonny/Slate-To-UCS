//! UCS taxonomy, filename builder and keyword extraction.
//!
//! A port of `ttucs.taxonomy` and `ttucs.text`. The Python implementation is the
//! reference; `tests/conformance.rs` proves the two agree on the golden corpus, so a
//! divergence is a test failure rather than a silent difference in a filename.
//!
//! ```
//! use slate_ucs::{Taxonomy, UcsFilename};
//!
//! let taxonomy = Taxonomy::bundled().expect("bundled taxonomy");
//! let entry = taxonomy.get("DOORCreak").expect("known CatID");
//! assert_eq!(entry.category_full(), "DOORS-CREAK");
//!
//! let name = UcsFilename::new("DOORCreak", "Wooden Door Creak Slow")
//!     .with_creator_id("NONE")
//!     .with_source_id("ThickAsThieves")
//!     .build()
//!     .expect("built filename");
//! assert_eq!(name, "DOORCreak_Wooden Door Creak Slow_NONE_ThickAsThieves");
//! ```

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod catid;
mod error;
mod filename;
mod keywords;
mod resources;
mod taxonomy;
mod text;

pub use catid::CatId;
pub use error::UcsError;
pub use filename::{parse_ucs_filename, sanitize_block, UcsFilename};
pub use keywords::make_keywords;
pub use resources::{UCS_ENTRY_COUNT, UCS_SHA256, UCS_VERSION};
pub use taxonomy::{Taxonomy, UcsEntry};
pub use text::{
    abbreviate, content_tokens, initials, make_fx_name, split_description, strip_take_numbers,
};
