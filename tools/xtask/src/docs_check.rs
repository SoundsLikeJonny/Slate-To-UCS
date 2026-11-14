//! `cargo xtask docs-check`: fail on broken relative links in Markdown.

use std::path::{Path, PathBuf};

use crate::repo_root;

/// Run the documentation link check.
///
/// # Errors
///
/// Returns a message listing every broken relative link.
pub fn run(_args: &[String]) -> Result<(), String> {
    check_in(&repo_root())
}

/// Check every Markdown file under `root`, plus the root-level ones.
///
/// # Errors
///
/// Returns a message listing every broken relative link, one per line.
pub fn check_in(root: &Path) -> Result<(), String> {
    let mut problems: Vec<String> = Vec::new();
    for file in markdown_files(root) {
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        for target in relative_links(&text) {
            let resolved = resolve(&file, &target);
            if !resolved.exists() {
                problems.push(format!(
                    "{}: {} -> {}",
                    file.strip_prefix(root).unwrap_or(&file).display(),
                    target,
                    resolved.display()
                ));
            }
        }
    }
    if problems.is_empty() {
        println!("documentation links resolve");
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Every Markdown file under `root`, excluding build and dependency directories.
fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out, 0);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if matches!(
                name.as_ref(),
                "target" | "node_modules" | ".git" | "dist" | "build" | ".abacusai" | ".idea"
            ) {
                continue;
            }
            collect(&path, out, depth + 1);
        } else if name.ends_with(".md") {
            out.push(path);
        }
    }
}

/// Relative link targets in a Markdown document, excluding anchors, URLs and
/// absolute paths.
#[must_use]
pub fn relative_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == ']' && bytes[i + 1] == '(' {
            let start = i + 2;
            let mut end = start;
            while end < bytes.len() && bytes[end] != ')' {
                end += 1;
            }
            if end < bytes.len() {
                let target: String = bytes[start..end].iter().collect();
                let target = target.split('#').next().unwrap_or("").trim().to_string();
                if !target.is_empty()
                    && !target.starts_with("http://")
                    && !target.starts_with("https://")
                    && !target.starts_with("mailto:")
                    && !target.starts_with('/')
                {
                    out.push(target);
                }
            }
            i = end;
        }
        i += 1;
    }
    out
}

/// Resolve a relative link against the directory of the file that contains it.
fn resolve(file: &Path, target: &str) -> PathBuf {
    let base = file.parent().unwrap_or(Path::new("."));
    base.join(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_relative_links_only() {
        let text = "[a](PLAN.md) [b](https://x/y) [c](#anchor) [d](/abs) [e](docs/X.md#frag)";

        let links = relative_links(text);

        assert_eq!(links, vec!["PLAN.md", "docs/X.md"]);
    }

    #[test]
    fn ignores_image_free_text() {
        assert!(relative_links("no links here").is_empty());
    }

    #[test]
    fn resolves_against_the_containing_directory() {
        let resolved = resolve(Path::new("/repo/docs/PLAN.md"), "LAYOUT.md");

        assert_eq!(resolved, PathBuf::from("/repo/docs/LAYOUT.md"));
    }

    #[test]
    fn reports_a_broken_link() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.md"), "[x](missing.md)\n").expect("write");

        let err = check_in(dir.path()).expect_err("broken");

        assert!(err.contains("missing.md"), "got {err}");
    }

    #[test]
    fn accepts_a_working_link() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.md"), "[x](b.md)\n").expect("write");
        std::fs::write(dir.path().join("b.md"), "# b\n").expect("write");

        assert!(check_in(dir.path()).is_ok());
    }

    #[test]
    fn the_repository_documentation_resolves() {
        assert!(check_in(&repo_root()).is_ok());
    }
}
