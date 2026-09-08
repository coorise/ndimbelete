//! Tiny className helper (React `cn` style).

pub fn cn(parts: &[&str]) -> String {
    parts
        .iter()
        .copied()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
