//! `cargo xtask docs-check`: fail on broken relative links in Markdown.

use std::path::{Path, PathBuf};

use crate::repo_root;

/// Run the documentation link check.
///
/// # Errors
///
/// Returns a message listing every broken relative link.
pub fn run(_args: &[String]) -> Result<(), String> {
    let root = repo_root();
    check_in(&root)?;
    check_summary(&root)
}

/// Check that `docs/SUMMARY.md` lists every document and nothing missing.
///
/// mdBook refuses to build without a `SUMMARY.md`, and a document missing from it is
/// invisible on the published site even though it reads fine on GitHub. The `docs`
/// workflow only runs on `main`, so a break here is not caught by a pull request
/// unless this check catches it first.
///
/// # Errors
///
/// Returns a message listing every unlisted document and every dangling entry.
pub fn check_summary(root: &Path) -> Result<(), String> {
    let docs = root.join("docs");
    let summary = docs.join("SUMMARY.md");
    let text =
        std::fs::read_to_string(&summary).map_err(|e| format!("{}: {e}", summary.display()))?;
    let listed: Vec<PathBuf> = relative_links(&text)
        .into_iter()
        .map(|target| docs.join(target))
        .collect();

    let mut problems: Vec<String> = Vec::new();
    for entry in &listed {
        if !entry.exists() {
            problems.push(format!(
                "SUMMARY.md lists a missing file: {}",
                entry.display()
            ));
        }
    }
    for file in markdown_files(&docs) {
        if file.file_name().is_some_and(|name| name == "SUMMARY.md") {
            continue;
        }
        if !listed.contains(&file) {
            problems.push(format!(
                "SUMMARY.md does not list {}",
                file.strip_prefix(&docs).unwrap_or(&file).display()
            ));
        }
    }
    if problems.is_empty() {
        println!("documentation summary lists {} documents", listed.len());
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
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

    #[test]
    fn the_repository_summary_lists_every_document() {
        assert!(check_summary(&repo_root()).is_ok());
    }

    #[test]
    fn a_summary_that_omits_a_document_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let docs = dir.path().join("docs");
        std::fs::create_dir_all(&docs).expect("mkdir");
        std::fs::write(docs.join("SUMMARY.md"), "# Summary\n\n- [A](a.md)\n").expect("write");
        std::fs::write(docs.join("a.md"), "# a\n").expect("write");
        std::fs::write(docs.join("b.md"), "# b\n").expect("write");

        let err = check_summary(dir.path()).expect_err("unlisted");

        assert!(err.contains("does not list b.md"), "got {err}");
        assert!(!err.contains("a.md"), "got {err}");
    }

    #[test]
    fn a_summary_entry_with_no_file_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let docs = dir.path().join("docs");
        std::fs::create_dir_all(&docs).expect("mkdir");
        std::fs::write(docs.join("SUMMARY.md"), "- [Gone](gone.md)\n").expect("write");

        let err = check_summary(dir.path()).expect_err("dangling");

        assert!(err.contains("lists a missing file"), "got {err}");
        assert!(err.contains("gone.md"), "got {err}");
    }

    #[test]
    fn a_missing_summary_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("docs")).expect("mkdir");

        let err = check_summary(dir.path()).expect_err("no summary");

        assert!(err.contains("SUMMARY.md"), "got {err}");
    }

    #[test]
    fn a_complete_summary_passes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let docs = dir.path().join("docs");
        std::fs::create_dir_all(docs.join("adr")).expect("mkdir");
        std::fs::write(docs.join("SUMMARY.md"), "- [A](a.md)\n  - [D](adr/d.md)\n").expect("write");
        std::fs::write(docs.join("a.md"), "# a\n").expect("write");
        std::fs::write(docs.join("adr/d.md"), "# d\n").expect("write");

        assert!(check_summary(dir.path()).is_ok());
    }
}
