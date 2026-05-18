use anyhow::Context;
use std::path::Path;

/// Read a file as UTF-8, with helpful error context.
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> anyhow::Result<String> {
    let p = path.as_ref();
    let bytes = tokio::fs::read(p)
        .await
        .with_context(|| format!("reading {}", p.display()))?;
    String::from_utf8(bytes).with_context(|| format!("{} is not valid UTF-8", p.display()))
}

/// Write a string to a file, with helpful error context.
pub async fn write_string<P: AsRef<Path>>(path: P, contents: &str) -> anyhow::Result<()> {
    let p = path.as_ref();
    tokio::fs::write(p, contents)
        .await
        .with_context(|| format!("writing {}", p.display()))
}

/// Convert any string into a stable kebab/snake slug.
pub fn slugify<T: AsRef<str>>(name: T) -> String {
    let mut out = String::new();
    let mut prev_was_sep = true;
    for ch in name.as_ref().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_was_sep = false;
        } else if !prev_was_sep {
            out.push('_');
            prev_was_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Noobini Pizzanini"), "noobini_pizzanini");
        assert_eq!(slugify("V.I.P Member"), "v_i_p_member");
        assert_eq!(slugify("  weird---name!! "), "weird_name");
        assert_eq!(slugify("Already_snake"), "already_snake");
    }
}
