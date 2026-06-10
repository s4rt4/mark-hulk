//! Markdown -> preview blocks.
//!
//! Mark-Hulk renders natively (no web engine). Most content becomes Pango
//! markup shown in labels; tables become real GtkGrid widgets, so the preview
//! is built from a sequence of `Block`s rather than one string. Code blocks are
//! syntax-highlighted with syntect.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// A renderable preview block.
pub enum Block {
    /// A run of non-table content, already as Pango markup.
    Markup(String),
    /// A table; each cell holds Pango markup. `head` may be empty.
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_DARK: OnceLock<Theme> = OnceLock::new();
static THEME_LIGHT: OnceLock<Theme> = OnceLock::new();

/// Mirrors the app's theme: when true, code blocks highlight with a light
/// syntect palette. Set via [`set_light`] on startup and on every theme switch.
static LIGHT: AtomicBool = AtomicBool::new(false);

/// Select the light or dark code-highlight palette to match the app theme.
pub fn set_light(light: bool) {
    LIGHT.store(light, Ordering::Relaxed);
}

fn syntaxes() -> &'static SyntaxSet {
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn load_theme(name: &str) -> Theme {
    let ts = ThemeSet::load_defaults();
    ts.themes
        .get(name)
        .or_else(|| ts.themes.values().next())
        .cloned()
        .expect("at least one default theme")
}

fn theme() -> &'static Theme {
    if LIGHT.load(Ordering::Relaxed) {
        THEME_LIGHT.get_or_init(|| load_theme("base16-ocean.light"))
    } else {
        THEME_DARK.get_or_init(|| load_theme("base16-ocean.dark"))
    }
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

fn flush(out: &mut String, blocks: &mut Vec<Block>) {
    let trimmed = out.trim();
    if !trimmed.is_empty() {
        blocks.push(Block::Markup(trimmed.to_string()));
    }
    out.clear();
}

/// Parse markdown into a sequence of preview blocks.
pub fn render_blocks(source: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(source, opts);

    let mut blocks: Vec<Block> = Vec::new();
    let mut out = String::new();
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    let mut in_code = false;
    let mut code_buf = String::new();
    let mut code_lang = String::new();

    // Table accumulation.
    let mut in_table = false;
    let mut in_head = false;
    let mut head: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => out.push_str(&format!(
                    "<span size=\"{}\" weight=\"bold\">",
                    heading_size(level)
                )),
                Tag::Paragraph => {}
                Tag::Emphasis => target(in_table, &mut out, &mut cell).push_str("<i>"),
                Tag::Strong => target(in_table, &mut out, &mut cell).push_str("<b>"),
                Tag::Strikethrough => target(in_table, &mut out, &mut cell).push_str("<s>"),
                Tag::Link { dest_url, .. } => target(in_table, &mut out, &mut cell)
                    .push_str(&format!("<a href=\"{}\">", esc(&dest_url))),
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
                Tag::Table(_) => {
                    flush(&mut out, &mut blocks);
                    in_table = true;
                    head.clear();
                    rows.clear();
                }
                Tag::TableHead => {
                    in_head = true;
                    row.clear();
                }
                Tag::TableRow => row.clear(),
                Tag::TableCell => cell.clear(),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => out.push_str("</span>\n\n"),
                TagEnd::Paragraph => out.push_str("\n\n"),
                TagEnd::Emphasis => target(in_table, &mut out, &mut cell).push_str("</i>"),
                TagEnd::Strong => target(in_table, &mut out, &mut cell).push_str("</b>"),
                TagEnd::Strikethrough => target(in_table, &mut out, &mut cell).push_str("</s>"),
                TagEnd::Link => target(in_table, &mut out, &mut cell).push_str("</a>"),
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
                TagEnd::TableCell => row.push(std::mem::take(&mut cell)),
                TagEnd::TableRow => {
                    if !in_head {
                        rows.push(std::mem::take(&mut row));
                    }
                }
                TagEnd::TableHead => {
                    head = std::mem::take(&mut row);
                    in_head = false;
                }
                TagEnd::Table => {
                    blocks.push(Block::Table {
                        head: std::mem::take(&mut head),
                        rows: std::mem::take(&mut rows),
                    });
                    in_table = false;
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_code {
                    code_buf.push_str(&t);
                } else {
                    target(in_table, &mut out, &mut cell).push_str(&esc(&t));
                }
            }
            Event::Code(t) => {
                let buf = target(in_table, &mut out, &mut cell);
                buf.push_str("<tt>");
                buf.push_str(&esc(&t));
                buf.push_str("</tt>");
            }
            Event::SoftBreak => target(in_table, &mut out, &mut cell).push(' '),
            Event::HardBreak => target(in_table, &mut out, &mut cell).push('\n'),
            Event::Rule => out.push_str("\n──────────────────────\n\n"),
            Event::TaskListMarker(checked) => {
                target(in_table, &mut out, &mut cell)
                    .push_str(if checked { "☑  " } else { "☐  " });
            }
            _ => {}
        }
    }

    flush(&mut out, &mut blocks);
    blocks
}

/// Pick the buffer that inline content should go to.
fn target<'a>(in_table: bool, out: &'a mut String, cell: &'a mut String) -> &'a mut String {
    if in_table {
        cell
    } else {
        out
    }
}
