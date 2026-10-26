//! What a page's text says about itself: frontmatter, title, type, the pages it
//! links to and the Sources it cites. Computed on every write and stored next
//! to the text, so graph, backlinks, lint and "which pages cite this Source"
//! are queries instead of scans.

use serde_json::{Map, Value};

/// Bump when the rules below change; pages stored with an older revision are
/// re-derived on the next start.
pub(crate) const DERIVE_REV: i16 = 1;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct PageFacts {
    /// The parsed frontmatter as a JSON object; empty when absent or invalid.
    pub frontmatter: Map<String, Value>,
    pub title: Option<String>,
    pub page_type: Option<String>,
    /// Link targets in the form the resolver compares: see [`link_ref`].
    pub links: Vec<String>,
    /// The `sources:` entries as written.
    pub sources: Vec<String>,
}

/// Pages that take part in the link graph: Markdown under `wiki/`.
pub(crate) fn is_wiki_page(path: &str) -> bool {
    path.starts_with("wiki/") && path.ends_with(".md")
}

pub(crate) fn derive(path: &str, content: &str) -> PageFacts {
    let (frontmatter, body) = split_frontmatter(content);
    let title = frontmatter
        .get("title")
        .and_then(scalar)
        .or_else(|| heading(body))
        .or_else(|| stem_title(path));
    let page_type = frontmatter.get("type").and_then(scalar);

    // Same sweep the desktop reader does over the whole text, so a page links
    // to exactly what the links panel showed before.
    let mut links = Vec::new();
    for target in wikilinks(content)
        .into_iter()
        .chain(frontmatter.get("related").map(strings).unwrap_or_default())
    {
        if let Some(target) = link_ref(&target) {
            if !links.contains(&target) {
                links.push(target);
            }
        }
    }
    let mut sources = Vec::new();
    for source in frontmatter.get("sources").map(strings).unwrap_or_default() {
        let source = source.trim().to_string();
        if !source.is_empty() && !sources.contains(&source) {
            sources.push(source);
        }
    }
    PageFacts {
        frontmatter,
        title,
        page_type,
        links,
        sources,
    }
}

/// Splits a leading `---` block off the text and parses it. A block that is
/// not valid YAML yields an empty map but is still cut off the body.
fn split_frontmatter(content: &str) -> (Map<String, Value>, &str) {
    let text = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return (Map::new(), text);
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let payload = &rest[..offset];
            return (parse_yaml(payload), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (Map::new(), text)
}

fn parse_yaml(payload: &str) -> Map<String, Value> {
    let parse = |text: &str| serde_yaml_ng::from_str::<Value>(text).ok();
    match parse(payload).or_else(|| parse(&quote_wikilink_lists(payload))) {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Models write `related: [[a]], [[b]]`, which is not YAML. Quote each link so
/// the line becomes a list of strings.
fn quote_wikilink_lists(payload: &str) -> String {
    payload
        .lines()
        .map(|line| match line.split_once(':') {
            Some((key, value))
                if value.trim_start().starts_with("[[") && value.contains("]],") =>
            {
                let items: Vec<String> = value
                    .split("],")
                    .map(|item| item.trim().trim_end_matches(']').trim_start_matches('[').trim())
                    .map(|item| format!("\"[[{item}]]\""))
                    .collect();
                format!("{key}: [{}]", items.join(", "))
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn scalar(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => return None,
    };
    (!text.is_empty()).then_some(text)
}

/// A string or a list of them, as a list.
fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().filter_map(scalar).collect(),
        other => scalar(other).into_iter().collect(),
    }
}

fn heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(|title| title.trim().to_string())
        .filter(|title| !title.is_empty())
}

fn stem_title(path: &str) -> Option<String> {
    let name = path.rsplit('/').next()?;
    Some(name.trim_end_matches(".md").replace('-', " ")).filter(|t| !t.is_empty())
}

/// The text of every `[[target]]` / `[[target|label]]`.
fn wikilinks(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        out.push(rest[..end].split('|').next().unwrap_or_default().to_string());
        rest = &rest[end + 2..];
    }
    out
}

/// The form a link target is stored and compared in: `[[ ]]` and an `#anchor`
/// removed, `\` turned into `/`. A target with a `/` names a path; one
/// without names a file and is matched by file name anywhere in the wiki.
pub(crate) fn link_ref(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix("[[")
        .and_then(|r| r.strip_suffix("]]"))
        .unwrap_or(raw);
    let target = raw.split('#').next().unwrap_or_default().trim();
    let target = target.replace('\\', "/");
    (!target.is_empty()).then_some(target)
}

/// The page a link resolves to among `paths` (live pages, any order): a
/// target with a `/` must be a page path; a bare one matches a file name
/// anywhere in `wiki/`, the first path in byte order winning. The same rule
/// as the SQL that keeps `page_links` up to date.
pub(crate) fn resolve_link<'a>(paths: &[&'a str], target_ref: &str) -> Option<&'a str> {
    let mut graph = paths.iter().copied().filter(|p| is_wiki_page(p));
    if target_ref.contains('/') {
        return graph.find(|p| *p == target_ref);
    }
    let name = if target_ref.ends_with(".md") {
        target_ref.to_string()
    } else {
        format!("{target_ref}.md")
    };
    let nested = format!("/{name}");
    graph.filter(|p| *p == name || p.ends_with(&nested)).min()
}

/// Whether the `sources:` entry `source_ref` cites the Source stored at
/// `source_path`: the same path, the path without `raw/sources/`, or the file
/// name at the end of it.
pub(crate) fn cites(source_ref: &str, source_path: &str) -> bool {
    source_ref == source_path
        || format!("raw/sources/{source_ref}") == source_path
        || source_path.ends_with(&format!("/{source_ref}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_title_type_and_sources_are_read() {
        let facts = derive(
            "wiki/entities/attention.md",
            "---\ntype: concept\ntitle: Attention\nsources: [a.pdf, \"b c.pdf\"]\ntags:\n  - nlp\n---\n# Heading\nbody",
        );
        assert_eq!(facts.title.as_deref(), Some("Attention"));
        assert_eq!(facts.page_type.as_deref(), Some("concept"));
        assert_eq!(facts.sources, ["a.pdf", "b c.pdf"]);
        assert_eq!(facts.frontmatter["tags"][0], "nlp");
    }

    #[test]
    fn title_falls_back_to_heading_then_file_name() {
        assert_eq!(
            derive("wiki/a.md", "intro\n# The Heading\n").title.as_deref(),
            Some("The Heading")
        );
        assert_eq!(
            derive("wiki/vector-database.md", "plain").title.as_deref(),
            Some("vector database")
        );
    }

    #[test]
    fn links_come_from_the_body_and_from_related() {
        let facts = derive(
            "wiki/a.md",
            "---\nrelated:\n  - \"[[c]]\"\n  - d\n---\nsee [[b|label]], [[wiki/x/y.md#sec]] and [[b]]",
        );
        for want in ["b", "c", "d", "wiki/x/y.md"] {
            assert!(facts.links.iter().any(|l| l == want), "{want} in {:?}", facts.links);
        }
        assert_eq!(facts.links.iter().filter(|l| *l == "b").count(), 1);
    }

    #[test]
    fn a_bare_wikilink_list_in_related_is_repaired() {
        let facts = derive("wiki/a.md", "---\ntitle: T\nrelated: [[a]], [[b]]\n---\nbody");
        assert_eq!(facts.title.as_deref(), Some("T"));
        assert!(facts.links.contains(&"a".to_string()) && facts.links.contains(&"b".to_string()));
    }

    #[test]
    fn invalid_yaml_keeps_the_links_and_drops_the_frontmatter() {
        let facts = derive("wiki/a.md", "---\ntitle: [unclosed\n---\n# Real\n[[b]]");
        assert!(facts.frontmatter.is_empty());
        assert_eq!(facts.title.as_deref(), Some("Real"));
        assert_eq!(facts.links, ["b"]);
    }

    #[test]
    fn crlf_and_bom_are_tolerated_and_no_frontmatter_is_fine() {
        let facts = derive("wiki/a.md", "\u{feff}---\r\ntype: x\r\n---\r\nbody");
        assert_eq!(facts.page_type.as_deref(), Some("x"));
        let none = derive("wiki/a.md", "# Only a heading\n---\nnot frontmatter\n---");
        assert!(none.frontmatter.is_empty());
    }

    #[test]
    fn link_refs_are_normalised() {
        assert_eq!(link_ref(" [[Foo#bar]] ").as_deref(), Some("Foo"));
        assert_eq!(link_ref("wiki\\a\\b.md").as_deref(), Some("wiki/a/b.md"));
        assert_eq!(link_ref("#only-anchor"), None);
    }
}
