//! Markdown -> Pango markup renderer.
//!
//! Mark-Hulk renders natively (no web engine), so the preview is a single
//! GtkLabel with Pango markup. pulldown-cmark drives a small event loop that
//! emits markup. This is the lightweight v0 renderer; richer block widgets
//! (tables, syntax-highlighted code frames) can replace it incrementally.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Escape a string for use inside Pango markup (text or attribute value).
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn heading_size(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "180%",
        HeadingLevel::H2 => "150%",
        HeadingLevel::H3 => "125%",
        HeadingLevel::H4 => "110%",
        HeadingLevel::H5 => "100%",
        HeadingLevel::H6 => "90%",
    }
}

/// Render markdown source to a Pango markup string.
pub fn to_pango_markup(source: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(source, opts);

    let mut out = String::new();
    let mut in_code = false;
    // Ordered-list counters; None = bullet list.
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    out.push_str(&format!(
                        "<span size=\"{}\" weight=\"bold\">",
                        heading_size(level)
                    ));
                }
                Tag::Paragraph => {}
                Tag::Emphasis => out.push_str("<i>"),
                Tag::Strong => out.push_str("<b>"),
                Tag::Strikethrough => out.push_str("<s>"),
                Tag::Link { dest_url, .. } => {
                    out.push_str(&format!("<a href=\"{}\">", esc(&dest_url)));
                }
                Tag::List(start) => list_stack.push(start),
                Tag::Item => {
                    let depth = list_stack.len().saturating_sub(1);
                    let indent = "    ".repeat(depth);
                    let marker = match list_stack.last_mut() {
                        Some(Some(n)) => {
                            let m = format!("{}. ", n);
                            *n += 1;
                            m
                        }
                        _ => "•  ".to_string(),
                    };
                    out.push_str(&format!("{}{}", indent, marker));
                }
                Tag::CodeBlock(_kind) => {
                    in_code = true;
                    out.push_str("<tt>");
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => out.push_str("</span>\n\n"),
                TagEnd::Paragraph => out.push_str("\n\n"),
                TagEnd::Emphasis => out.push_str("</i>"),
                TagEnd::Strong => out.push_str("</b>"),
                TagEnd::Strikethrough => out.push_str("</s>"),
                TagEnd::Link => out.push_str("</a>"),
                TagEnd::Item => out.push('\n'),
                TagEnd::List(_) => {
                    list_stack.pop();
                    out.push('\n');
                }
                TagEnd::CodeBlock => {
                    in_code = false;
                    out.push_str("</tt>\n\n");
                }
                _ => {}
            },
            Event::Text(t) => out.push_str(&esc(&t)),
            Event::Code(t) => {
                out.push_str("<tt>");
                out.push_str(&esc(&t));
                out.push_str("</tt>");
            }
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("\n──────────────────────\n\n"),
            Event::TaskListMarker(checked) => {
                out.push_str(if checked { "☑  " } else { "☐  " });
            }
            // HTML, footnote refs, and anything else: ignored in v0.
            _ => {
                let _ = in_code;
            }
        }
    }

    // Avoid Pango complaining about an empty string.
    let trimmed = out.trim_end().to_string();
    if trimmed.is_empty() {
        " ".to_string()
    } else {
        trimmed
    }
}

/// Used by CodeBlockKind matching elsewhere if needed.
#[allow(dead_code)]
fn lang_of(kind: &CodeBlockKind) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(),
        CodeBlockKind::Indented => String::new(),
    }
}
