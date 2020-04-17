//! Mastodon API models
//!
//! This crate contains the data structures sent to and returned from, the Mastodon API
//!
//! # Example
//!
//! ```
//! use mastodon_api_models::field::Field;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let field_json = r#"{
//!     "name": "foo",
//!     "value": "bar"
//! }"#;
//! let field: Field = serde_json::from_str(field_json)?;
//!
//! assert_eq!(field, Field::builder().name("foo").value("bar").build().unwrap());
//! #    Ok(())
//! # }
//! ```
