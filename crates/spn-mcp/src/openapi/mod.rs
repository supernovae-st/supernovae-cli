//! OpenAPI 3.0+ parsing and conversion to API wrapper config.
//!
//! Converts OpenAPI specs to [`ApiConfig`] for use with spn-mcp.

mod parser;

pub use parser::{parse_openapi, OpenApiError, OpenApiSpec};
