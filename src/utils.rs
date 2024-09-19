/// Checks if the given `snippet` starts with any of the provided `prefixes`.
///
/// # Arguments
///
/// * `snippet` - A string slice that will be checked.
/// * `prefixes` - A slice of string slices representing the prefixes to check against.
///
/// # Returns
///
/// * `Some(prefix)` if the `snippet` starts with any of the `prefixes`, where `prefix` is the
///  first matching prefix.
/// * `None` otherwise.
///
/// # Examples
///
/// ```
/// let snippet = "hello world";
/// let prefixes = ["he", "wo"];
/// assert_eq!(start_with_one_of(snippet, &prefixes), Some("he"));
/// ```
pub fn start_with_one_of<'a>(snippet: &str, prefixes: &[&'a str]) -> Option<&'a str> {
    for prefix in prefixes {
        if snippet.starts_with(prefix) {
            return Some(prefix);
        }
    }
    None
}

/// Checks if the given `snippet` ends with any of the provided `suffixes`.
///
/// # Arguments
///
/// * `snippet` - A string slice that will be checked.
/// * `suffixes` - A slice of string slices representing the suffixes to check against.
///
/// # Returns
///
/// * `Some(suffix)` if the `snippet` ends with any of the `suffixes`, where `suffix` is the
/// first matching suffix.
/// * `None` otherwise.
///
/// # Examples
///
/// ```
/// let snippet = "hello world";
/// let suffixes = ["ld", "lo"];
/// assert_eq!(end_with_one_of(snippet, &suffixes), Some("ld"));
/// ```
pub fn end_with_one_of<'a>(snippet: &str, suffixes: &[&'a str]) -> Option<&'a str> {
    for suffix in suffixes {
        if snippet.ends_with(suffix) {
            return Some(suffix);
        }
    }
    None
}
