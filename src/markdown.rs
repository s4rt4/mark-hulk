//! Markdown -> preview blocks.
//!
//! Mark-Hulk renders natively (no web engine). Most content becomes Pango
//! markup shown in labels; tables become real GtkGrid widgets, so the preview
//! is built from a sequence of `Block`s rather than one string. Code blocks are
//! syntax-highlighted with syntect.

use std::path::{Path, PathBuf};
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
    /// A heading. `markup` is the styled Pango markup, `text` the plain text for
    /// the outline, `line` the 1-based source line so the outline can jump to it.
    Heading {
        level: u8,
        markup: String,
        text: String,
        line: usize,
    },
    /// A fenced/indented code block, already highlighted as Pango markup
    /// (wrapped in `<tt>`). Kept separate so the preview can give it its own
    /// horizontally-scrollable container instead of clipping long lines.
    Code(String),
    /// An image. `path` is the resolved local file (None for remote URLs, which
    /// we don't fetch); `alt` is the alt text shown when the image can't load.
    Image {
        path: Option<PathBuf>,
        url: String,
        alt: String,
    },
    /// A table; each cell holds Pango markup. `head` may be empty.
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// A heading for the document outline.
pub struct OutlineItem {
    pub level: u8,
    pub text: String,
    pub line: usize,
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

fn heading_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn lang_of(kind: &CodeBlockKind) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or("").to_string(),
        CodeBlockKind::Indented => String::new(),
    }
}

/// Resolve an image URL to a local file path, or `None` for remote URLs.
fn resolve_image(url: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:") {
        return None;
    }
    let raw = url.strip_prefix("file://").unwrap_or(url);
    let p = Path::new(raw);
    if p.is_absolute() {
        Some(p.to_path_buf())
    } else {
        base_dir.map(|d| d.join(raw))
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

/// Byte offset -> 1-based line number, using precomputed line-start offsets.
fn line_at(line_starts: &[usize], offset: usize) -> usize {
    line_starts.partition_point(|&s| s <= offset).max(1)
}

fn parser_options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts
}

/// Parse markdown into a sequence of preview blocks. `base_dir` resolves
/// relative image paths (typically the open document's directory).
pub fn render_blocks(source: &str, base_dir: Option<&Path>) -> Vec<Block> {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    let parser = Parser::new_ext(source, parser_options());

    let mut blocks: Vec<Block> = Vec::new();
    let mut out = String::new();
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    let mut in_code = false;
    let mut code_buf = String::new();
    let mut code_lang = String::new();

    // Heading accumulation: headings become their own block so the outline can
    // jump to them and the preview can scroll to the matching widget.
    let mut in_heading = false;
    let mut head_markup = String::new();
    let mut head_text = String::new();
    let mut head_level: u8 = 1;
    let mut head_line: usize = 1;

    // Image accumulation.
    let mut in_image = false;
    let mut img_url = String::new();
    let mut img_alt = String::new();

    // Table accumulation.
    let mut in_table = false;
    let mut in_head = false;
    let mut head: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush(&mut out, &mut blocks);
                    in_heading = true;
                    head_markup.clear();
                    head_text.clear();
                    head_level = heading_num(level);
                    head_line = line_at(&line_starts, range.start);
                    head_markup.push_str(&format!(
                        "<span size=\"{}\" weight=\"bold\">",
                        heading_size(level)
                    ));
                }
                Tag::Paragraph => {}
                Tag::Emphasis => target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                    .push_str("<i>"),
                Tag::Strong => target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                    .push_str("<b>"),
                Tag::Strikethrough => {
                    target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                        .push_str("<s>")
                }
                Tag::Link { dest_url, .. } => {
                    target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                        .push_str(&format!("<a href=\"{}\">", esc(&dest_url)))
                }
                Tag::Image { dest_url, .. } => {
                    flush(&mut out, &mut blocks);
                    in_image = true;
                    img_url = dest_url.to_string();
                    img_alt.clear();
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
                TagEnd::Heading(_) => {
                    head_markup.push_str("</span>");
                    blocks.push(Block::Heading {
                        level: head_level,
                        markup: std::mem::take(&mut head_markup),
                        text: head_text.trim().to_string(),
                        line: head_line,
                    });
                    in_heading = false;
                }
                TagEnd::Paragraph => out.push_str("\n\n"),
                TagEnd::Emphasis => {
                    target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                        .push_str("</i>")
                }
                TagEnd::Strong => target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                    .push_str("</b>"),
                TagEnd::Strikethrough => {
                    target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                        .push_str("</s>")
                }
                TagEnd::Link => target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                    .push_str("</a>"),
                TagEnd::Image => {
                    in_image = false;
                    let path = resolve_image(&img_url, base_dir);
                    blocks.push(Block::Image {
                        path,
                        url: std::mem::take(&mut img_url),
                        alt: std::mem::take(&mut img_alt),
                    });
                }
                TagEnd::Item => out.push('\n'),
                TagEnd::List(_) => {
                    list_stack.pop();
                    out.push('\n');
                }
                TagEnd::CodeBlock => {
                    in_code = false;
                    flush(&mut out, &mut blocks);
                    blocks.push(Block::Code(highlight_code(&code_buf, &code_lang)));
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
                } else if in_image {
                    img_alt.push_str(&t);
                } else {
                    if in_heading {
                        head_text.push_str(&t);
                    }
                    target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                        .push_str(&esc(&t));
                }
            }
            Event::Code(t) => {
                if in_heading {
                    head_text.push_str(&t);
                }
                let buf = target(in_heading, in_table, &mut out, &mut cell, &mut head_markup);
                buf.push_str("<tt>");
                buf.push_str(&esc(&t));
                buf.push_str("</tt>");
            }
            Event::SoftBreak => {
                target(in_heading, in_table, &mut out, &mut cell, &mut head_markup).push(' ')
            }
            Event::HardBreak => {
                target(in_heading, in_table, &mut out, &mut cell, &mut head_markup).push('\n')
            }
            Event::Rule => out.push_str("\n──────────────────────\n\n"),
            Event::TaskListMarker(checked) => {
                target(in_heading, in_table, &mut out, &mut cell, &mut head_markup)
                    .push_str(if checked { "☑  " } else { "☐  " });
            }
            _ => {}
        }
    }

    flush(&mut out, &mut blocks);
    blocks
}

/// Extract just the headings, for the document outline.
pub fn outline(source: &str) -> Vec<OutlineItem> {
    render_blocks(source, None)
        .into_iter()
        .filter_map(|b| match b {
            Block::Heading {
                level, text, line, ..
            } if !text.is_empty() => Some(OutlineItem { level, text, line }),
            _ => None,
        })
        .collect()
}

/// Render the document to a standalone HTML string for export.
pub fn to_html(source: &str, title: &str) -> String {
    let mut body = String::new();
    let parser = Parser::new_ext(source, parser_options());
    pulldown_cmark::html::push_html(&mut body, parser);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title}</title>\n<style>\n{CSS}\n</style>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        title = esc_html(title),
        body = body,
        CSS = HTML_CSS,
    )
}

/// Flatten the document to a single Pango markup string for PDF export.
pub fn to_pango(source: &str, base_dir: Option<&Path>) -> String {
    let mut out = String::new();
    for block in render_blocks(source, base_dir) {
        match block {
            Block::Markup(m) | Block::Code(m) => {
                out.push_str(&m);
                out.push_str("\n\n");
            }
            Block::Heading { markup, .. } => {
                out.push_str(&markup);
                out.push_str("\n\n");
            }
            Block::Image { alt, url, .. } => {
                out.push_str(&format!("<i>[image: {}]</i>\n\n", esc(&format!("{alt} {url}"))));
            }
            Block::Table { head, rows } => {
                out.push_str("<tt>");
                if !head.is_empty() {
                    out.push_str(&head.join("\t"));
                    out.push('\n');
                }
                for row in rows {
                    out.push_str(&row.join("\t"));
                    out.push('\n');
                }
                out.push_str("</tt>\n\n");
            }
        }
    }
    out
}

/// Minimal HTML escape for text nodes used in the export template.
fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const HTML_CSS: &str = "\
body { max-width: 46rem; margin: 2rem auto; padding: 0 1rem; \
  font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #1a1a1a; }
pre { background: #f4f4f4; padding: 0.8rem 1rem; border-radius: 6px; overflow-x: auto; }
code { background: #f4f4f4; padding: 0.1rem 0.3rem; border-radius: 4px; font-size: 0.95em; }
pre code { background: none; padding: 0; }
table { border-collapse: collapse; }
th, td { border: 1px solid #ccc; padding: 0.3rem 0.7rem; }
img { max-width: 100%; }
blockquote { border-left: 3px solid #ccc; margin: 0; padding-left: 1rem; color: #555; }
a { color: #2a7d4f; }";

/// Pick the buffer that inline content should go to.
fn target<'a>(
    in_heading: bool,
    in_table: bool,
    out: &'a mut String,
    cell: &'a mut String,
    head_markup: &'a mut String,
) -> &'a mut String {
    if in_heading {
        head_markup
    } else if in_table {
        cell
    } else {
        out
    }
}
