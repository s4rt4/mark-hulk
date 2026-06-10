//! Mark-Hulk — a fast, lightweight native GTK4 Markdown viewer & editor.
//!
//! No web engine: the UI is GTK4 + libadwaita, the editor is GtkSourceView,
//! and the preview is rendered to Pango markup. The goal is an instant cold
//! start and a tiny memory footprint.

mod fs;
mod markdown;
mod style;
mod watcher;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use notify::RecommendedWatcher;
use sourceview5::prelude::*;

const APP_ID: &str = "com.sarta.mark-hulk";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Preview,
    Split,
    Edit,
}

/// Per-tab document: its own editor, preview and state.
struct Doc {
    buffer: sourceview5::Buffer,
    view: sourceview5::View,
    preview_box: gtk::Box,
    editor_scroll: gtk::ScrolledWindow,
    preview_scroll: gtk::ScrolledWindow,
    page: RefCell<Option<adw::TabPage>>,
    path: RefCell<Option<PathBuf>>,
    dirty: Cell<bool>,
    loading: Cell<bool>,
}

/// The whole application: shared widgets and state.
struct App {
    window: adw::ApplicationWindow,
    tab_view: adw::TabView,
    sidebar_scroll: gtk::ScrolledWindow,
    sidebar_list: gtk::Box,
    sidebar_btn: gtk::ToggleButton,
    css_provider: gtk::CssProvider,
    theme: RefCell<String>,
    root: RefCell<Option<PathBuf>>,
    mode: Cell<Mode>,
    docs: RefCell<Vec<Rc<Doc>>>,
    watcher: RefCell<Option<RecommendedWatcher>>,
    reload_tx: async_channel::Sender<PathBuf>,
    preview_btn: gtk::ToggleButton,
    split_btn: gtk::ToggleButton,
    edit_btn: gtk::ToggleButton,
    search_revealer: gtk::Revealer,
    search_entry: gtk::SearchEntry,
    search_results: gtk::ListBox,
    search_hits: RefCell<Vec<(PathBuf, usize)>>,
}

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    // Build once on startup; activate/open just present and load files.
    let holder: Rc<RefCell<Option<Rc<App>>>> = Rc::new(RefCell::new(None));

    {
        let holder = holder.clone();
        app.connect_startup(move |app| {
            *holder.borrow_mut() = Some(build_app(app));
        });
    }
    {
        let holder = holder.clone();
        app.connect_activate(move |_| {
            if let Some(app) = holder.borrow().as_ref() {
                app.window.present();
            }
        });
    }
    {
        let holder = holder.clone();
        app.connect_open(move |_, files, _hint| {
            if let Some(app) = holder.borrow().as_ref() {
                for file in files {
                    if let Some(path) = file.path() {
                        open_path(app, &path);
                    }
                }
                app.window.present();
            }
        });
    }

    app.run()
}

fn build_app(gapp: &adw::Application) -> Rc<App> {
    style::install_schemes();
    let theme = style::load_saved_theme();
    adw::StyleManager::default().set_color_scheme(color_scheme_for(&theme));
    markdown::set_light(style::is_light(&theme));
    let css_provider = style::make_provider();
    style::apply_css(&css_provider, &theme);

    let window = adw::ApplicationWindow::builder()
        .application(gapp)
        .title("Mark-Hulk")
        .default_width(1100)
        .default_height(720)
        .width_request(680)
        .height_request(480)
        .build();

    // --- Sidebar ----------------------------------------------------------
    let sidebar_list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();
    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .child(&sidebar_list)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .width_request(240)
        .css_classes(vec!["mh-sidebar".to_string()])
        .build();

    // --- Tabs -------------------------------------------------------------
    let tab_view = adw::TabView::new();
    let tab_bar = adw::TabBar::builder().view(&tab_view).build();

    // --- Search panel -----------------------------------------------------
    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search workspace…")
        .build();
    let search_results = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(vec!["boxed-list".to_string()])
        .build();
    let results_scroll = gtk::ScrolledWindow::builder()
        .child(&search_results)
        .vexpand(false)
        .max_content_height(280)
        .propagate_natural_height(true)
        .build();
    let search_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    search_box.append(&search_entry);
    search_box.append(&results_scroll);
    let search_revealer = gtk::Revealer::builder()
        .child(&search_box)
        .reveal_child(false)
        .build();

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content_box.append(&tab_bar);
    content_box.append(&search_revealer);
    content_box.append(&tab_view);

    // --- Outer split ------------------------------------------------------
    let main_paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&sidebar_scroll)
        .end_child(&content_box)
        .position(240)
        .resize_start_child(false)
        .build();

    // --- Header bar -------------------------------------------------------
    let header = adw::HeaderBar::new();
    let sidebar_btn = gtk::ToggleButton::builder()
        .icon_name("view-list-symbolic")
        .active(true)
        .tooltip_text("Toggle sidebar (Ctrl+B)")
        .build();
    header.pack_start(&sidebar_btn);
    let open_folder_btn = icon_button("folder-open-symbolic", "Open folder");
    let open_file_btn = icon_button("document-open-symbolic", "Open file");
    let new_btn = icon_button("document-new-symbolic", "New file (Ctrl+N)");
    let search_btn = gtk::ToggleButton::builder()
        .icon_name("system-search-symbolic")
        .tooltip_text("Search workspace (Ctrl+Shift+F)")
        .build();
    header.pack_start(&open_folder_btn);
    header.pack_start(&open_file_btn);
    header.pack_start(&new_btn);
    header.pack_start(&search_btn);

    let save_btn = icon_button("document-save-symbolic", "Save (Ctrl+S)");
    header.pack_end(&save_btn);

    let preview_btn = gtk::ToggleButton::with_label("Preview");
    let split_btn = gtk::ToggleButton::with_label("Split");
    let edit_btn = gtk::ToggleButton::with_label("Edit");
    split_btn.set_group(Some(&preview_btn));
    edit_btn.set_group(Some(&preview_btn));
    preview_btn.set_active(true);
    let mode_box = gtk::Box::builder()
        .css_classes(vec!["linked".to_string()])
        .build();
    mode_box.append(&preview_btn);
    mode_box.append(&split_btn);
    mode_box.append(&edit_btn);
    header.pack_end(&mode_box);

    let theme_menu = gio::Menu::new();
    theme_menu.append(Some("Forest Sage"), Some("win.theme::forest"));
    theme_menu.append(Some("Dark Emerald"), Some("win.theme::emerald"));
    theme_menu.append(Some("Light Sage"), Some("win.theme::light"));
    let theme_btn = gtk::MenuButton::builder()
        .icon_name("applications-graphics-symbolic")
        .tooltip_text("Theme")
        .menu_model(&theme_menu)
        .build();
    header.pack_end(&theme_btn);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&main_paned));
    window.set_content(Some(&toolbar_view));

    let (reload_tx, reload_rx) = async_channel::unbounded::<PathBuf>();

    let app = Rc::new(App {
        window: window.clone(),
        tab_view: tab_view.clone(),
        sidebar_scroll,
        sidebar_list,
        sidebar_btn: sidebar_btn.clone(),
        css_provider,
        theme: RefCell::new(theme),
        root: RefCell::new(None),
        mode: Cell::new(Mode::Preview),
        docs: RefCell::new(Vec::new()),
        watcher: RefCell::new(None),
        reload_tx,
        preview_btn: preview_btn.clone(),
        split_btn: split_btn.clone(),
        edit_btn: edit_btn.clone(),
        search_revealer,
        search_entry: search_entry.clone(),
        search_results: search_results.clone(),
        search_hits: RefCell::new(Vec::new()),
    });

    wire_signals(
        &app,
        &open_folder_btn,
        &open_file_btn,
        &new_btn,
        &save_btn,
        &search_btn,
        &preview_btn,
        &split_btn,
        &edit_btn,
        &search_entry,
        &search_results,
        gapp,
    );

    // Sidebar show/hide.
    {
        let app = app.clone();
        sidebar_btn.connect_toggled(move |b| app.sidebar_scroll.set_visible(b.is_active()));
    }

    // Theme switcher: a stateful "win.theme" action backs the header menu.
    {
        let initial = app.theme.borrow().clone();
        let theme_action = gio::SimpleAction::new_stateful(
            "theme",
            Some(glib::VariantTy::STRING),
            &initial.to_variant(),
        );
        {
            let app = app.clone();
            theme_action.connect_activate(move |action, param| {
                if let Some(theme) = param.and_then(|p| p.str()) {
                    action.set_state(&theme.to_variant());
                    set_theme(&app, theme);
                }
            });
        }
        app.window.add_action(&theme_action);
    }

    // Receive file-change events on the UI thread and reload the active doc.
    {
        let app = app.clone();
        glib::spawn_future_local(async move {
            while let Ok(changed) = reload_rx.recv().await {
                if let Some(doc) = current_doc(&app) {
                    let matches = doc.path.borrow().as_deref() == Some(changed.as_path());
                    if matches && !doc.dirty.get() {
                        if let Ok(text) = fs::read_file(&changed) {
                            doc.loading.set(true);
                            doc.buffer.set_text(&text);
                            doc.loading.set(false);
                            refresh_preview(&doc);
                        }
                    }
                }
            }
        });
    }

    // Open with one empty tab.
    let doc = new_doc(&app);
    select_doc(&app, &doc);

    app
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name(icon);
    b.set_tooltip_text(Some(tooltip));
    b
}

#[allow(clippy::too_many_arguments)]
fn wire_signals(
    app: &Rc<App>,
    open_folder_btn: &gtk::Button,
    open_file_btn: &gtk::Button,
    new_btn: &gtk::Button,
    save_btn: &gtk::Button,
    search_btn: &gtk::ToggleButton,
    preview_btn: &gtk::ToggleButton,
    split_btn: &gtk::ToggleButton,
    edit_btn: &gtk::ToggleButton,
    search_entry: &gtk::SearchEntry,
    search_results: &gtk::ListBox,
    gapp: &adw::Application,
) {
    {
        let app = app.clone();
        open_folder_btn.connect_clicked(move |_| open_folder(&app));
    }
    {
        let app = app.clone();
        open_file_btn.connect_clicked(move |_| open_file(&app));
    }
    {
        let app = app.clone();
        new_btn.connect_clicked(move |_| {
            let d = new_doc(&app);
            select_doc(&app, &d);
        });
    }
    {
        let app = app.clone();
        save_btn.connect_clicked(move |_| save(&app));
    }
    {
        let app = app.clone();
        search_btn.connect_toggled(move |b| {
            app.search_revealer.set_reveal_child(b.is_active());
            if b.is_active() {
                app.search_entry.grab_focus();
            }
        });
    }
    for (btn, mode) in [
        (preview_btn, Mode::Preview),
        (split_btn, Mode::Split),
        (edit_btn, Mode::Edit),
    ] {
        let app = app.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() {
                app.mode.set(mode);
                if let Some(d) = current_doc(&app) {
                    apply_mode(&app, &d);
                }
            }
        });
    }

    // Re-apply mode, re-watch, and retitle when the active tab changes.
    {
        let app = app.clone();
        let tv = app.tab_view.clone();
        tv.connect_selected_page_notify(move |_| on_tab_changed(&app));
    }
    {
        let app = app.clone();
        let tv = app.tab_view.clone();
        tv.connect_page_detached(move |_view, page, _| {
            app.docs
                .borrow_mut()
                .retain(|d| d.page.borrow().as_ref() != Some(page));
            if app.tab_view.n_pages() == 0 {
                let d = new_doc(&app);
                select_doc(&app, &d);
            }
        });
    }

    // Search.
    {
        let app = app.clone();
        search_entry.connect_activate(move |_| run_search(&app));
    }
    {
        let app = app.clone();
        search_results.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            let hit = app.search_hits.borrow().get(idx).cloned();
            if let Some((path, line)) = hit {
                open_path(&app, &path);
                if let Some(d) = current_doc(&app) {
                    goto_line(&d, line);
                }
                app.search_revealer.set_reveal_child(false);
            }
        });
    }

    // Keyboard shortcuts.
    add_action(app, gapp, "save", "<Primary>s", |app| save(app));
    add_action(app, gapp, "new", "<Primary>n", |app| {
        let d = new_doc(app);
        select_doc(app, &d);
    });
    add_action(app, gapp, "toggle-sidebar", "<Primary>b", |app| {
        app.sidebar_btn.set_active(!app.sidebar_btn.is_active());
    });
    add_action(app, gapp, "search", "<Primary><Shift>f", |app| {
        let r = !app.search_revealer.reveals_child();
        app.search_revealer.set_reveal_child(r);
        if r {
            app.search_entry.grab_focus();
        }
    });
    add_action(app, gapp, "mode-preview", "<Primary>1", |app| {
        app.preview_btn.set_active(true)
    });
    add_action(app, gapp, "mode-split", "<Primary>2", |app| {
        app.split_btn.set_active(true)
    });
    add_action(app, gapp, "mode-edit", "<Primary>3", |app| {
        app.edit_btn.set_active(true)
    });
}

fn add_action(
    app: &Rc<App>,
    gapp: &adw::Application,
    name: &str,
    accel: &str,
    f: impl Fn(&Rc<App>) + 'static,
) {
    let action = gio::SimpleAction::new(name, None);
    let appc = app.clone();
    action.connect_activate(move |_, _| f(&appc));
    app.window.add_action(&action);
    gapp.set_accels_for_action(&format!("win.{name}"), &[accel]);
}

/// Build a fresh, empty document tab and return it.
fn new_doc(app: &Rc<App>) -> Rc<Doc> {
    let buffer = sourceview5::Buffer::builder().build();
    if let Some(lang) = sourceview5::LanguageManager::default().language("markdown") {
        buffer.set_language(Some(&lang));
    }
    if let Some(scheme) = style::scheme_for(&app.theme.borrow()) {
        buffer.set_style_scheme(Some(&scheme));
    }
    let view = sourceview5::View::with_buffer(&buffer);
    view.set_monospace(true);
    view.set_show_line_numbers(true);
    view.set_highlight_current_line(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_top_margin(8);
    view.set_bottom_margin(8);
    view.set_left_margin(8);
    view.set_right_margin(8);
    let editor_scroll = gtk::ScrolledWindow::builder()
        .child(&view)
        .hexpand(true)
        .vexpand(true)
        .build();

    let preview_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(20)
        .margin_end(20)
        .valign(gtk::Align::Start)
        .build();
    let preview_scroll = gtk::ScrolledWindow::builder()
        .child(&preview_box)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .vexpand(true)
        .build();

    let content = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&editor_scroll)
        .end_child(&preview_scroll)
        .position(540)
        .build();

    let doc = Rc::new(Doc {
        buffer: buffer.clone(),
        view,
        preview_box,
        editor_scroll,
        preview_scroll,
        page: RefCell::new(None),
        path: RefCell::new(None),
        dirty: Cell::new(false),
        loading: Cell::new(false),
    });

    let page = app.tab_view.append(&content);
    page.set_title("Untitled");
    *doc.page.borrow_mut() = Some(page);
    app.docs.borrow_mut().push(doc.clone());

    {
        let doc = doc.clone();
        buffer.connect_changed(move |_| {
            refresh_preview(&doc);
            if !doc.loading.get() && !doc.dirty.get() {
                doc.dirty.set(true);
                update_tab_title(&doc);
            }
        });
    }

    refresh_preview(&doc);
    apply_mode(app, &doc);
    doc
}

fn select_doc(app: &Rc<App>, doc: &Rc<Doc>) {
    if let Some(page) = doc.page.borrow().clone() {
        app.tab_view.set_selected_page(&page);
    }
}

fn current_doc(app: &Rc<App>) -> Option<Rc<Doc>> {
    let page = app.tab_view.selected_page()?;
    app.docs
        .borrow()
        .iter()
        .find(|d| d.page.borrow().as_ref() == Some(&page))
        .cloned()
}

fn on_tab_changed(app: &Rc<App>) {
    if let Some(doc) = current_doc(app) {
        apply_mode(app, &doc);
        set_window_title(app, &doc);
        let path = doc.path.borrow().clone();
        if let Some(path) = path {
            start_watch(app, &path);
        }
    }
}

/// Map a theme to the libadwaita color-scheme it should force.
fn color_scheme_for(theme: &str) -> adw::ColorScheme {
    if style::is_light(theme) {
        adw::ColorScheme::ForceLight
    } else {
        adw::ColorScheme::ForceDark
    }
}

fn set_theme(app: &Rc<App>, theme: &str) {
    adw::StyleManager::default().set_color_scheme(color_scheme_for(theme));
    markdown::set_light(style::is_light(theme));
    style::apply_css(&app.css_provider, theme);
    if let Some(scheme) = style::scheme_for(theme) {
        for doc in app.docs.borrow().iter() {
            doc.buffer.set_style_scheme(Some(&scheme));
        }
    }
    *app.theme.borrow_mut() = theme.to_string();
    style::save_theme(theme);
    // Code-block colors are baked into the preview markup by syntect, so a
    // light<->dark switch needs every open preview rebuilt.
    for doc in app.docs.borrow().iter() {
        refresh_preview(doc);
    }
}

fn apply_mode(app: &Rc<App>, doc: &Rc<Doc>) {
    let (edit, preview) = match app.mode.get() {
        Mode::Preview => (false, true),
        Mode::Split => (true, true),
        Mode::Edit => (true, false),
    };
    doc.editor_scroll.set_visible(edit);
    doc.preview_scroll.set_visible(preview);
}

fn refresh_preview(doc: &Rc<Doc>) {
    let buf = &doc.buffer;
    let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);

    while let Some(child) = doc.preview_box.first_child() {
        doc.preview_box.remove(&child);
    }

    let blocks = markdown::render_blocks(&text);
    if blocks.is_empty() {
        doc.preview_box
            .append(&preview_text("<i>Open a folder or a file to begin.</i>"));
        return;
    }
    for block in blocks {
        match block {
            markdown::Block::Markup(m) => doc.preview_box.append(&preview_text(&m)),
            markdown::Block::Table { head, rows } => {
                doc.preview_box.append(&preview_table(&head, &rows))
            }
        }
    }
}

fn preview_text(markup: &str) -> gtk::Label {
    let label = gtk::Label::builder()
        .use_markup(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.0)
        .yalign(0.0)
        .selectable(true)
        .halign(gtk::Align::Fill)
        .css_classes(vec!["mh-preview".to_string()])
        .build();
    label.set_markup(markup);
    label
}

fn preview_table(head: &[String], rows: &[Vec<String>]) -> gtk::Widget {
    let grid = gtk::Grid::builder()
        .column_spacing(18)
        .row_spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    let ncols = head
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));

    for (c, cell) in head.iter().enumerate() {
        grid.attach(&cell_label(cell, true), c as i32, 0, 1, 1);
    }
    if ncols > 0 && !head.is_empty() {
        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
        grid.attach(&sep, 0, 1, ncols as i32, 1);
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            grid.attach(&cell_label(cell, false), c as i32, (r + 2) as i32, 1, 1);
        }
    }

    gtk::Frame::builder()
        .child(&grid)
        .halign(gtk::Align::Start)
        .margin_top(2)
        .margin_bottom(4)
        .build()
        .upcast()
}

fn cell_label(markup: &str, bold: bool) -> gtk::Label {
    let m = if markup.trim().is_empty() {
        " ".to_string()
    } else if bold {
        format!("<b>{markup}</b>")
    } else {
        markup.to_string()
    };
    // Table cells don't wrap — wrapping labels inside a grid trigger GTK
    // height-for-width measurement warnings, and short cells read better.
    let label = gtk::Label::builder()
        .use_markup(true)
        .xalign(0.0)
        .selectable(true)
        .css_classes(vec!["mh-preview".to_string()])
        .build();
    label.set_markup(&m);
    label
}

fn update_tab_title(doc: &Rc<Doc>) {
    let name = doc
        .path
        .borrow()
        .as_ref()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Untitled".to_string());
    let title = if doc.dirty.get() {
        format!("• {name}")
    } else {
        name
    };
    if let Some(page) = doc.page.borrow().as_ref() {
        page.set_title(&title);
    }
}

fn set_window_title(app: &Rc<App>, doc: &Rc<Doc>) {
    let name = doc
        .path
        .borrow()
        .as_ref()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let title = match name {
        Some(n) => format!("{n} — Mark-Hulk"),
        None => "Mark-Hulk".to_string(),
    };
    app.window.set_title(Some(&title));
}

fn load_into_doc(app: &Rc<App>, doc: &Rc<Doc>, path: &Path) {
    match fs::read_file(path) {
        Ok(text) => {
            doc.loading.set(true);
            doc.buffer.set_text(&text);
            doc.loading.set(false);
            *doc.path.borrow_mut() = Some(path.to_path_buf());
            doc.dirty.set(false);
            update_tab_title(doc);
            set_window_title(app, doc);
            refresh_preview(doc);
            start_watch(app, path);
        }
        Err(e) => eprintln!("Failed to read {}: {e}", path.display()),
    }
}

/// Open `path`: focus its existing tab, reuse an empty tab, or open a new one.
fn open_path(app: &Rc<App>, path: &Path) {
    let existing = app
        .docs
        .borrow()
        .iter()
        .find(|d| d.path.borrow().as_deref() == Some(path))
        .cloned();
    if let Some(doc) = existing {
        select_doc(app, &doc);
        return;
    }

    let reuse = current_doc(app)
        .filter(|d| d.path.borrow().is_none() && d.buffer.char_count() == 0);
    let doc = match reuse {
        Some(d) => d,
        None => {
            let d = new_doc(app);
            select_doc(app, &d);
            d
        }
    };
    load_into_doc(app, &doc, path);
}

fn start_watch(app: &Rc<App>, path: &Path) {
    match watcher::watch_file(path, app.reload_tx.clone()) {
        Ok(w) => *app.watcher.borrow_mut() = Some(w),
        Err(e) => eprintln!("Failed to watch {}: {e}", path.display()),
    }
}

fn save(app: &Rc<App>) {
    let Some(doc) = current_doc(app) else { return };
    let path = doc.path.borrow().clone();
    if let Some(path) = path {
        let buf = &doc.buffer;
        let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
        match fs::write_file(&path, &text) {
            Ok(()) => {
                doc.dirty.set(false);
                update_tab_title(&doc);
            }
            Err(e) => eprintln!("Failed to save {}: {e}", path.display()),
        }
    }
}

fn goto_line(doc: &Rc<Doc>, line: usize) {
    let buf = &doc.buffer;
    if let Some(mut iter) = buf.iter_at_line((line.saturating_sub(1)) as i32) {
        buf.place_cursor(&iter);
        doc.view.scroll_to_iter(&mut iter, 0.1, true, 0.0, 0.4);
        doc.view.grab_focus();
    }
}

fn run_search(app: &Rc<App>) {
    while let Some(child) = app.search_results.first_child() {
        app.search_results.remove(&child);
    }
    app.search_hits.borrow_mut().clear();

    let root = app.root.borrow().clone();
    let query = app.search_entry.text().to_string();
    let Some(root) = root else {
        return;
    };

    for hit in fs::search_workspace(&root, &query) {
        let row_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_top(4)
            .margin_bottom(4)
            .margin_start(6)
            .margin_end(6)
            .build();
        let head = gtk::Label::builder()
            .label(format!("{}:{}", hit.name, hit.line))
            .xalign(0.0)
            .css_classes(vec!["heading".to_string()])
            .build();
        let text = gtk::Label::builder()
            .label(hit.text.clone())
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(vec!["dim-label".to_string()])
            .build();
        row_box.append(&head);
        row_box.append(&text);
        app.search_results.append(&row_box);
        app.search_hits.borrow_mut().push((hit.path, hit.line));
    }
}

fn open_folder(app: &Rc<App>) {
    let dialog = gtk::FileDialog::builder().title("Open folder").build();
    let app = app.clone();
    dialog.select_folder(Some(&app.window.clone()), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                populate_tree(&app, &path);
            }
        }
    });
}

fn open_file(app: &Rc<App>) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Markdown"));
    filter.add_pattern("*.md");
    filter.add_pattern("*.markdown");
    filter.add_pattern("*.mdx");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);

    let dialog = gtk::FileDialog::builder()
        .title("Open file")
        .filters(&filters)
        .build();
    let app = app.clone();
    dialog.open(Some(&app.window.clone()), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                open_path(&app, &path);
            }
        }
    });
}

fn populate_tree(app: &Rc<App>, root: &Path) {
    *app.root.borrow_mut() = Some(root.to_path_buf());

    while let Some(child) = app.sidebar_list.first_child() {
        app.sidebar_list.remove(&child);
    }

    if let Some(tree) = fs::build_tree(root) {
        for child in &tree.children {
            let w = tree_widget(app, child);
            app.sidebar_list.append(&w);
        }
    }
}

fn tree_widget(app: &Rc<App>, node: &fs::TreeNode) -> gtk::Widget {
    if node.is_dir {
        let expander = gtk::Expander::new(Some(&node.name));
        let inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_start(12)
            .build();
        for child in &node.children {
            inner.append(&tree_widget(app, child));
        }
        expander.set_child(Some(&inner));
        expander.upcast()
    } else {
        let label = gtk::Label::new(Some(&node.name));
        label.set_xalign(0.0);
        let button = gtk::Button::builder()
            .child(&label)
            .has_frame(false)
            .hexpand(true)
            .build();
        let path = node.path.clone();
        let app = app.clone();
        button.connect_clicked(move |_| open_path(&app, &path));
        button.upcast()
    }
}
