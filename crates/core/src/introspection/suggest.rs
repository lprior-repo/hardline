//! Name suggestion functionality
//!
//! This module provides the suggest_name function for generating
//! unique session names based on patterns.

use crate::error::{Error, Result};

use super::query::SuggestNameQuery;

/// Parse a name pattern and suggest next available name
///
/// Pattern format: `prefix-{n}` or `{n}-suffix` where {n} is a number placeholder
#[allow(clippy::literal_string_with_formatting_args)]
pub fn suggest_name(pattern: &str, existing_names: &[String]) -> Result<SuggestNameQuery> {
    if !pattern.contains("{n}") {
        return Err(Error::ValidationError(
            "Pattern must contain {n} placeholder".to_string(),
        ));
    }

    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::ValidationError(
            "Pattern must contain exactly one {n} placeholder".to_string(),
        ));
    }

    let prefix = parts
        .first()
        .ok_or_else(|| Error::ValidationError("Pattern parts missing".to_string()))?;
    let suffix = parts
        .get(1)
        .ok_or_else(|| Error::ValidationError("Pattern parts missing suffix".to_string()))?;

    let (used_numbers, matching): (Vec<usize>, Vec<String>) = existing_names
        .iter()
        .filter(|name| name.starts_with(prefix) && name.ends_with(suffix))
        .filter_map(|name| {
            let num_part = name
                .get(prefix.len()..name.len().saturating_sub(suffix.len()))
                .map_or("", |s| s);
            num_part.parse::<usize>().ok().map(|n| (n, name.clone()))
        })
        .unzip();

    let next_n = (1..=used_numbers.len() + 2)
        .find(|n| !used_numbers.contains(n))
        .map_or(1, |n| n);

    let suggested = pattern.replace("{n}", &next_n.to_string());

    Ok(SuggestNameQuery {
        pattern: pattern.to_string(),
        suggested,
        next_available_n: next_n,
        existing_matches: matching,
    })
}
