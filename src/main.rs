//! Mark-Hulk — a fast, lightweight native GTK4 Markdown viewer & editor.
//!
//! No web engine: the UI is GTK4 + libadwaita, the editor is GtkSourceView,
//! and the preview is rendered to Pango markup. The goal is an instant cold
//! start and a tiny memory footprint.

mod export;
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
    /// Heading widgets in preview order, so the outline can scroll to one.
    headings: RefCell<Vec<gtk::Widget>>,
    /// Per-buffer find/replace context (lazily created), bound to the shared
    /// `App::search_settings`.
    search_context: RefCell<Option<sourceview5::SearchContext>>,
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
    // In-document find & replace.
    find_revealer: gtk::Revealer,
    find_entry: gtk::SearchEntry,
    replace_entry: gtk::Entry,
    find_settings: sourceview5::SearchSettings,
    // Document outline.
    outline_box: gtk::Box,
    outline_btn: gtk::MenuButton,
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

    // --- Find & replace bar (in current document) ------------------------
    let find_entry = gtk::SearchEntry::builder()
        .placeholder_text("Find")
        .hexpand(true)
        .build();
    let find_prev = icon_button("go-up-symbolic", "Previous match");
    let find_next = icon_button("go-down-symbolic", "Next match");
    let replace_entry = gtk::Entry::builder()
        .placeholder_text("Replace with")
        .hexpand(true)
        .build();
    let replace_one_btn = gtk::Button::with_label("Replace");
    let replace_all_btn = gtk::Button::with_label("All");
    let find_close = icon_button("window-close-symbolic", "Close (Esc)");
    let find_grid = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(8)
        .margin_end(8)
        .build();
    find_grid.append(&find_entry);
    find_grid.append(&find_prev);
    find_grid.append(&find_next);
    find_grid.append(&replace_entry);
    find_grid.append(&replace_one_btn);
    find_grid.append(&replace_all_btn);
    find_grid.append(&find_close);
    let find_revealer = gtk::Revealer::builder()
        .child(&find_grid)
        .reveal_child(false)
        .build();
    let find_settings = sourceview5::SearchSettings::new();
    find_settings.set_wrap_around(true);

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    content_box.append(&tab_bar);
    content_box.append(&search_revealer);
    content_box.append(&find_revealer);
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
    let outline_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(1)
        .margin_top(4)
        .margin_bottom(4)
        .margin_start(4)
        .margin_end(4)
        .build();
    let outline_pop = gtk::Popover::builder()
        .child(
            &gtk::ScrolledWindow::builder()
                .child(&outline_box)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .max_content_height(420)
                .propagate_natural_height(true)
                .width_request(260)
                .build(),
        )
        .build();
    let outline_btn = gtk::MenuButton::builder()
        .icon_name("view-list-ordered-symbolic")
        .tooltip_text("Document outline")
        .popover(&outline_pop)
        .build();

    header.pack_start(&open_folder_btn);
    header.pack_start(&open_file_btn);
    header.pack_start(&new_btn);
    header.pack_start(&search_btn);
    header.pack_start(&outline_btn);

    let save_btn = icon_button("document-save-symbolic", "Save (Ctrl+S)");
    header.pack_end(&save_btn);

    let export_menu = gio::Menu::new();
    export_menu.append(Some("Export HTML…"), Some("win.export-html"));
    export_menu.append(Some("Export PDF…"), Some("win.export-pdf"));
    let export_btn = gtk::MenuButton::builder()
        .icon_name("document-send-symbolic")
        .tooltip_text("Export")
        .menu_model(&export_menu)
        .build();
    header.pack_end(&export_btn);

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
        find_revealer,
        find_entry: find_entry.clone(),
        replace_entry: replace_entry.clone(),
        find_settings,
        outline_box,
        outline_btn: outline_btn.clone(),
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

    // Outline: rebuild the heading list each time the popover opens.
    {
        let app = app.clone();
        outline_btn.connect_active_notify(move |b| {
            if b.is_active() {
                rebuild_outline(&app);
            }
        });
    }

    // Find & replace bar.
    {
        let app = app.clone();
        find_entry.connect_search_changed(move |e| {
            app.find_settings.set_search_text(Some(&e.text()));
        });
    }
    {
        let app = app.clone();
        find_entry.connect_activate(move |_| find_step(&app, true));
    }
    {
        let app = app.clone();
        find_next.connect_clicked(move |_| find_step(&app, true));
    }
    {
        let app = app.clone();
        find_prev.connect_clicked(move |_| find_step(&app, false));
    }
    {
        let app = app.clone();
        replace_one_btn.connect_clicked(move |_| replace_one(&app));
    }
    {
        let app = app.clone();
        replace_all_btn.connect_clicked(move |_| replace_all(&app));
    }
    {
        let app = app.clone();
        find_close.connect_clicked(move |_| close_find(&app));
    }
    {
        // Esc closes the find bar.
        let app = app.clone();
        let revealer = app.find_revealer.clone();
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk::gdk::Key::Escape {
                close_find(&app);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        revealer.add_controller(key);
    }

    // Export actions, backing the header Export menu.
    {
        let app = app.clone();
        let win = app.window.clone();
        let action = gio::SimpleAction::new("export-html", None);
        action.connect_activate(move |_, _| export_dialog(&app, ExportKind::Html));
        win.add_action(&action);
    }
    {
        let app = app.clone();
        let win = app.window.clone();
        let action = gio::SimpleAction::new("export-pdf", None);
        action.connect_activate(move |_, _| export_dialog(&app, ExportKind::Pdf));
        win.add_action(&action);
    }

    // Confirm before closing a tab or the window with unsaved changes.
    {
        let app = app.clone();
        tab_view.connect_close_page(move |tv, page| {
            let doc = app
                .docs
                .borrow()
                .iter()
                .find(|d| d.page.borrow().as_ref() == Some(page))
                .cloned();
            match doc {
                Some(doc) if doc.dirty.get() => {
                    confirm_close_tab(&app, tv, page, &doc);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
    }
    {
        let app = app.clone();
        window.connect_close_request(move |_| {
            if app.docs.borrow().iter().any(|d| d.dirty.get()) {
                confirm_quit(&app);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
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
    add_action(app, gapp, "find", "<Primary>f", |app| open_find(app, false));
    add_action(app, gapp, "replace", "<Primary>h", |app| open_find(app, true));
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
        editor_scroll: editor_scroll.clone(),
        preview_scroll: preview_scroll.clone(),
        page: RefCell::new(None),
        path: RefCell::new(None),
        dirty: Cell::new(false),
        loading: Cell::new(false),
        headings: RefCell::new(Vec::new()),
        search_context: RefCell::new(None),
    });

    wire_scroll_sync(app, &editor_scroll, &preview_scroll);

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
    doc.headings.borrow_mut().clear();

    let base_dir = doc
        .path
        .borrow()
        .as_ref()
        .and_then(|p| p.parent().map(Path::to_path_buf));

    let blocks = markdown::render_blocks(&text, base_dir.as_deref());
    if blocks.is_empty() {
        doc.preview_box
            .append(&preview_text("<i>Open a folder or a file to begin.</i>"));
        return;
    }
    for block in blocks {
        match block {
            markdown::Block::Markup(m) => doc.preview_box.append(&preview_text(&m)),
            markdown::Block::Heading { markup, .. } => {
                let w = preview_text(&markup);
                doc.preview_box.append(&w);
                doc.headings.borrow_mut().push(w.upcast());
            }
            markdown::Block::Code(m) => doc.preview_box.append(&preview_code(&m)),
            markdown::Block::Image { path, url, alt } => {
                doc.preview_box.append(&preview_image(path.as_deref(), &url, &alt))
            }
            markdown::Block::Table { head, rows } => {
                doc.preview_box.append(&hscroll(&preview_table(&head, &rows)))
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

/// A code block: a non-wrapping monospace label so long lines keep their shape,
/// placed in its own horizontally-scrollable container via [`hscroll`].
fn preview_code(markup: &str) -> gtk::Widget {
    let label = gtk::Label::builder()
        .use_markup(true)
        .wrap(false)
        .xalign(0.0)
        .yalign(0.0)
        .selectable(true)
        .halign(gtk::Align::Start)
        .css_classes(vec!["mh-preview".to_string()])
        .build();
    label.set_markup(markup);
    hscroll(&label.upcast())
}

/// Wrap a non-wrapping child (code block, wide table) in a scrolled window that
/// scrolls horizontally on demand but never vertically — so it overflows into
/// its own scrollbar instead of clipping against the narrow preview pane. Height
/// follows the child's natural size (`propagate_natural_height`).
fn hscroll(child: &gtk::Widget) -> gtk::Widget {
    gtk::ScrolledWindow::builder()
        .child(child)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .halign(gtk::Align::Fill)
        .build()
        .upcast()
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

/// Render an image block: a scaled `GtkPicture` for a loadable local file, or
/// an italic alt-text placeholder for anything that can't be shown (missing
/// file, or a remote URL we don't fetch).
fn preview_image(path: Option<&Path>, url: &str, alt: &str) -> gtk::Widget {
    if let Some(p) = path {
        if p.exists() {
            if let Ok(pb) = gtk::gdk_pixbuf::Pixbuf::from_file(p) {
                let texture = gtk::gdk::Texture::for_pixbuf(&pb);
                let pic = gtk::Picture::for_paintable(&texture);
                pic.set_can_shrink(true);
                pic.set_halign(gtk::Align::Start);
                let (nw, nh) = (pb.width().max(1), pb.height().max(1));
                let maxw = 720;
                let (rw, rh) = if nw > maxw {
                    (maxw, (nh as f64 * maxw as f64 / nw as f64) as i32)
                } else {
                    (nw, nh)
                };
                pic.set_size_request(rw, rh);
                return pic.upcast();
            }
        }
    }
    let body = if url.is_empty() {
        format!("🖼 {}", glib::markup_escape_text(alt))
    } else {
        format!("🖼 {} ({})", glib::markup_escape_text(alt), glib::markup_escape_text(url))
    };
    preview_text(&format!("<i>{body}</i>")).upcast()
}

/// Proportional, two-way scroll linking between an editor and its preview,
/// active only in Split mode. A guard flag breaks the value-changed feedback.
fn wire_scroll_sync(app: &Rc<App>, editor: &gtk::ScrolledWindow, preview: &gtk::ScrolledWindow) {
    let guard = Rc::new(Cell::new(false));
    let ea = editor.vadjustment();
    let pa = preview.vadjustment();
    {
        let pa = pa.clone();
        let guard = guard.clone();
        let app = app.clone();
        ea.connect_value_changed(move |ea| {
            if guard.get() || app.mode.get() != Mode::Split {
                return;
            }
            guard.set(true);
            set_proportional(ea, &pa);
            guard.set(false);
        });
    }
    {
        let ea = ea.clone();
        let guard = guard.clone();
        let app = app.clone();
        pa.connect_value_changed(move |pa| {
            if guard.get() || app.mode.get() != Mode::Split {
                return;
            }
            guard.set(true);
            set_proportional(pa, &ea);
            guard.set(false);
        });
    }
}

fn set_proportional(from: &gtk::Adjustment, to: &gtk::Adjustment) {
    let denom = from.upper() - from.page_size();
    let frac = if denom > 0.0 { from.value() / denom } else { 0.0 };
    let tdenom = to.upper() - to.page_size();
    to.set_value(frac * tdenom);
}

/// Rebuild the outline popover from the active document's headings.
fn rebuild_outline(app: &Rc<App>) {
    while let Some(child) = app.outline_box.first_child() {
        app.outline_box.remove(&child);
    }
    let Some(doc) = current_doc(app) else { return };
    let buf = &doc.buffer;
    let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
    let items = markdown::outline(&text);
    if items.is_empty() {
        let l = gtk::Label::builder()
            .label("No headings")
            .css_classes(vec!["dim-label".to_string()])
            .margin_top(8)
            .margin_bottom(8)
            .build();
        app.outline_box.append(&l);
        return;
    }
    for (idx, item) in items.iter().enumerate() {
        let indent = "    ".repeat(item.level.saturating_sub(1) as usize);
        let label = gtk::Label::new(Some(&format!("{indent}{}", item.text)));
        label.set_xalign(0.0);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let btn = gtk::Button::builder()
            .child(&label)
            .has_frame(false)
            .build();
        let app2 = app.clone();
        let line = item.line;
        btn.connect_clicked(move |_| {
            if let Some(d) = current_doc(&app2) {
                goto_line(&d, line);
                scroll_preview_to_heading(&d, idx);
            }
            app2.outline_btn.popdown();
        });
        app.outline_box.append(&btn);
    }
}

/// Scroll the preview so the `idx`-th heading widget is near the top.
fn scroll_preview_to_heading(doc: &Rc<Doc>, idx: usize) {
    let headings = doc.headings.borrow();
    let Some(w) = headings.get(idx) else { return };
    let origin = gtk::graphene::Point::new(0.0, 0.0);
    if let Some(p) = w.compute_point(&doc.preview_box, &origin) {
        doc.preview_scroll.vadjustment().set_value(p.y() as f64);
    }
}

/// Lazily create (and cache) a find/replace context bound to the doc's buffer.
fn ensure_search_context(app: &Rc<App>, doc: &Rc<Doc>) -> sourceview5::SearchContext {
    if let Some(ctx) = doc.search_context.borrow().as_ref() {
        return ctx.clone();
    }
    let ctx = sourceview5::SearchContext::new(&doc.buffer, Some(&app.find_settings));
    *doc.search_context.borrow_mut() = Some(ctx.clone());
    ctx
}

fn find_step(app: &Rc<App>, forward: bool) {
    let Some(doc) = current_doc(app) else { return };
    let ctx = ensure_search_context(app, &doc);
    let buf = &doc.buffer;
    let (sel_start, sel_end) = buf.selection_bounds().unwrap_or_else(|| {
        let i = buf.iter_at_mark(&buf.get_insert());
        (i.clone(), i)
    });
    let found = if forward {
        ctx.forward(&sel_end)
    } else {
        ctx.backward(&sel_start)
    };
    if let Some((mut s, _e, _wrapped)) = found {
        buf.select_range(&s, &_e);
        doc.view.scroll_to_iter(&mut s, 0.1, false, 0.0, 0.5);
    }
}

fn replace_one(app: &Rc<App>) {
    let Some(doc) = current_doc(app) else { return };
    let ctx = ensure_search_context(app, &doc);
    let buf = &doc.buffer;
    let replacement = app.replace_entry.text();
    if let Some((mut start, mut end)) = buf.selection_bounds() {
        let _ = ctx.replace(&mut start, &mut end, replacement.as_str());
    }
    find_step(app, true);
}

fn replace_all(app: &Rc<App>) {
    let Some(doc) = current_doc(app) else { return };
    let ctx = ensure_search_context(app, &doc);
    let replacement = app.replace_entry.text();
    let _ = ctx.replace_all(replacement.as_str());
}

fn open_find(app: &Rc<App>, replace: bool) {
    app.find_revealer.set_reveal_child(true);
    if let Some(doc) = current_doc(app) {
        if let Some((s, e)) = doc.buffer.selection_bounds() {
            let sel = doc.buffer.text(&s, &e, false);
            if !sel.is_empty() && !sel.contains('\n') {
                app.find_entry.set_text(&sel);
                app.find_settings.set_search_text(Some(&sel));
            }
        }
    }
    if replace {
        app.replace_entry.grab_focus();
    } else {
        app.find_entry.grab_focus();
    }
}

fn close_find(app: &Rc<App>) {
    app.find_revealer.set_reveal_child(false);
    app.find_settings.set_search_text(None);
    if let Some(doc) = current_doc(app) {
        doc.view.grab_focus();
    }
}

#[derive(Clone, Copy)]
enum ExportKind {
    Html,
    Pdf,
}

fn export_dialog(app: &Rc<App>, kind: ExportKind) {
    let Some(doc) = current_doc(app) else { return };
    let buf = &doc.buffer;
    let source = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();
    let base = doc.path.borrow().clone();
    let title = base
        .as_ref()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Untitled".to_string());
    let base_dir = base.as_ref().and_then(|p| p.parent().map(Path::to_path_buf));
    let (ext, initial) = match kind {
        ExportKind::Html => ("html", format!("{title}.html")),
        ExportKind::Pdf => ("pdf", format!("{title}.pdf")),
    };
    let dialog = gtk::FileDialog::builder()
        .title("Export")
        .initial_name(initial)
        .build();
    dialog.save(Some(&app.window), gio::Cancellable::NONE, move |res| {
        let Ok(file) = res else { return };
        let Some(mut path) = file.path() else { return };
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        let result = match kind {
            ExportKind::Html => export::write_html(&path, &source, &title).map_err(|e| e.to_string()),
            ExportKind::Pdf => {
                export::write_pdf(&path, &markdown::to_pango(&source, base_dir.as_deref()))
                    .map_err(|e| e.to_string())
            }
        };
        if let Err(e) = result {
            eprintln!("Export failed: {e}");
        }
    });
}

/// Save the doc, prompting for a path if it has none, then run `on_done`.
fn ensure_saved(app: &Rc<App>, doc: &Rc<Doc>, on_done: impl Fn(bool) + 'static) {
    let path = doc.path.borrow().clone();
    if let Some(path) = path {
        let buf = &doc.buffer;
        let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
        match fs::write_file(&path, &text) {
            Ok(()) => {
                doc.dirty.set(false);
                update_tab_title(doc);
                on_done(true);
            }
            Err(e) => {
                eprintln!("Failed to save {}: {e}", path.display());
                on_done(false);
            }
        }
        return;
    }
    let dialog = gtk::FileDialog::builder()
        .title("Save As")
        .initial_name("Untitled.md")
        .build();
    let app = app.clone();
    let doc = doc.clone();
    let win = app.window.clone();
    dialog.save(Some(&win), gio::Cancellable::NONE, move |res| {
        let Ok(file) = res else {
            on_done(false);
            return;
        };
        let Some(path) = file.path() else {
            on_done(false);
            return;
        };
        let buf = &doc.buffer;
        let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
        match fs::write_file(&path, &text) {
            Ok(()) => {
                *doc.path.borrow_mut() = Some(path.clone());
                doc.dirty.set(false);
                update_tab_title(&doc);
                set_window_title(&app, &doc);
                start_watch(&app, &path);
                on_done(true);
            }
            Err(e) => {
                eprintln!("Failed to save {}: {e}", path.display());
                on_done(false);
            }
        }
    });
}

fn confirm_close_tab(app: &Rc<App>, tv: &adw::TabView, page: &adw::TabPage, doc: &Rc<Doc>) {
    let dialog = adw::MessageDialog::new(
        Some(&app.window),
        Some("Save changes?"),
        Some("This document has unsaved changes."),
    );
    dialog.add_responses(&[("cancel", "Cancel"), ("discard", "Discard"), ("save", "Save")]);
    dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("save"));
    dialog.set_close_response("cancel");
    let app = app.clone();
    let tv = tv.clone();
    let page = page.clone();
    let doc = doc.clone();
    dialog.connect_response(None, move |_, resp| match resp {
        "discard" => tv.close_page_finish(&page, true),
        "save" => {
            let tv = tv.clone();
            let page = page.clone();
            ensure_saved(&app, &doc, move |ok| tv.close_page_finish(&page, ok));
        }
        _ => tv.close_page_finish(&page, false),
    });
    dialog.present();
}

fn confirm_quit(app: &Rc<App>) {
    let dialog = adw::MessageDialog::new(
        Some(&app.window),
        Some("Unsaved changes"),
        Some("Some open documents have unsaved changes. Quit anyway?"),
    );
    dialog.add_responses(&[("cancel", "Cancel"), ("discard", "Discard & Quit")]);
    dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let app = app.clone();
    dialog.connect_response(None, move |_, resp| {
        if resp == "discard" {
            for d in app.docs.borrow().iter() {
                d.dirty.set(false);
            }
            app.window.close();
        }
    });
    dialog.present();
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
    ensure_saved(app, &doc, |_| {});
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
