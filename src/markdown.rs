//! Markdown -> Pango markup renderer.
//!
//! Mark-Hulk renders natively (no web engine), so the preview is a single
//! GtkLabel with Pango markup. pulldown-cmark drives a small event loop that
//! emits markup; code blocks are syntax-highlighted with syntect.

use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

fn syntaxes() -> &'static SyntaxSet {
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes
            .get("base16-ocean.dark")
            .or_else(|| ts.themes.values().next())
            .cloned()
            .expect("at least one default theme")
    })
}

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

fn lang_of(kind: &CodeBlockKind) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(),
        CodeBlockKind::Indented => String::new(),
    }
}

/// Syntax-highlight a code block into Pango markup (wrapped in `<tt>`).
fn highlight_code(code: &str, lang: &str) -> String {
    let ss = syntaxes();
    let syntax = ss
        .find_syntax_by_token(lang)
        .or_else(|| ss.find_syntax_by_extension(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut hl = HighlightLines::new(syntax, theme());
    let mut out = String::from("<tt>");
    for line in LinesWithEndings::from(code) {
        match hl.highlight_line(line, ss) {
            Ok(ranges) => {
                for (style, text) in ranges {
                    let c = style.foreground;
                    out.push_str(&format!(
                        "<span foreground=\"#{:02X}{:02X}{:02X}\">{}</span>",
                        c.r,
                        c.g,
                        c.b,
                        esc(text)
                    ));
                }
            }
            Err(_) => out.push_str(&esc(line)),
        }
    }
    out.push_str("</tt>");
    out
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
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    // Code-block capture state.
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut code_lang = String::new();

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
                Tag::CodeBlock(kind) => {
                    in_code = true;
                    code_buf.clear();
                    code_lang = lang_of(&kind);
                }
                // Tables: rendered as monospace, header row bold.
                Tag::Table(_) => out.push_str("\n<tt>"),
                Tag::TableHead => out.push_str("<b>"),
                Tag::TableRow => {}
                Tag::TableCell => {}
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
                    out.push_str(&highlight_code(&code_buf, &code_lang));
                    out.push_str("\n\n");
                }
                TagEnd::TableHead => out.push_str("</b>\n"),
                TagEnd::TableRow => out.push('\n'),
                TagEnd::TableCell => out.push_str("  │  "),
                TagEnd::Table => out.push_str("</tt>\n\n"),
                _ => {}
            },
            Event::Text(t) => {
                if in_code {
                    code_buf.push_str(&t);
                } else {
                    out.push_str(&esc(&t));
                }
            }
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
            _ => {}
        }
    }

    let trimmed = out.trim_end().to_string();
    if trimmed.is_empty() {
        " ".to_string()
    } else {
        trimmed
    }
}
