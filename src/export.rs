//! Document export: standalone HTML and PDF.
//!
//! Both stay true to the "no web engine" goal — HTML is generated directly from
//! the markdown, and PDF is laid out with Pango onto a Cairo PDF surface.

use std::path::Path;

use gtk::cairo;
use gtk::pango;

use crate::markdown;

/// Write the document as a standalone, styled HTML file.
pub fn write_html(path: &Path, source: &str, title: &str) -> std::io::Result<()> {
    std::fs::write(path, markdown::to_html(source, title))
}

/// A4 portrait, 72 dpi points.
const PAGE_W: f64 = 595.0;
const PAGE_H: f64 = 842.0;
const MARGIN: f64 = 54.0;

/// Write the document as a paginated PDF. `markup` is a single Pango markup
/// string (see [`markdown::to_pango`]); pages break on line boundaries.
pub fn write_pdf(path: &Path, markup: &str) -> Result<(), Box<dyn std::error::Error>> {
    let text_w = PAGE_W - 2.0 * MARGIN;
    let text_h = PAGE_H - 2.0 * MARGIN;

    let surface = cairo::PdfSurface::new(PAGE_W, PAGE_H, path)?;
    let cr = cairo::Context::new(&surface)?;

    let layout = pangocairo::functions::create_layout(&cr);
    layout.set_width((text_w * pango::SCALE as f64) as i32);
    layout.set_wrap(pango::WrapMode::WordChar);
    layout.set_font_description(Some(&pango::FontDescription::from_string("Sans 10")));
    layout.set_markup(markup);

    // Break the laid-out text into pages at line boundaries so no line is cut.
    let page_h_pu = (text_h * pango::SCALE as f64) as i32;
    let mut breaks: Vec<i32> = vec![0];
    let mut page_start = 0;
    let mut iter = layout.iter();
    loop {
        let (y0, y1) = iter.line_yrange();
        if y1 - page_start > page_h_pu {
            page_start = y0;
            breaks.push(y0);
        }
        if !iter.next_line() {
            break;
        }
    }

    for &start in &breaks {
        cr.save()?;
        cr.rectangle(MARGIN, MARGIN, text_w, text_h);
        cr.clip();
        cr.move_to(MARGIN, MARGIN - start as f64 / pango::SCALE as f64);
        pangocairo::functions::show_layout(&cr, &layout);
        cr.restore()?;
        cr.show_page()?;
    }

    surface.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "# Title\n\nA paragraph with **bold**.\n\n## Section\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n";

    #[test]
    fn html_is_standalone() {
        let html = markdown::to_html(SRC, "Doc");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<table>"));
    }

    #[test]
    fn pdf_writes_a_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("mark-hulk-export-test.pdf");
        let markup = markdown::to_pango(SRC, None);
        write_pdf(&path, &markup).expect("pdf export");
        let bytes = std::fs::read(&path).expect("read pdf");
        assert!(bytes.starts_with(b"%PDF"), "not a PDF file");
        assert!(bytes.len() > 500, "PDF suspiciously small");
        let _ = std::fs::remove_file(&path);
    }
}
