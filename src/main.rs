//! Mark-Hulk — a fast, lightweight native GTK4 Markdown viewer & editor.
//!
//! No web engine: the UI is GTK4 + libadwaita, the editor is GtkSourceView,
//! and the preview is rendered to Pango markup. The goal is an instant cold
//! start and a tiny memory footprint.

mod fs;
mod markdown;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adw::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};
use sourceview5::prelude::*;

const APP_ID: &str = "com.sarta.mark-hulk";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Preview,
    Split,
    Edit,
}

/// All long-lived widgets and the bit of app state, shared via `Rc`.
struct Ui {
    window: adw::ApplicationWindow,
    sidebar_list: gtk::Box,
    source_buffer: sourceview5::Buffer,
    preview_label: gtk::Label,
    editor_scroll: gtk::ScrolledWindow,
    preview_scroll: gtk::ScrolledWindow,
    current_path: RefCell<Option<PathBuf>>,
    root: RefCell<Option<PathBuf>>,
}

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    // Lean toward the dark heritage of the original themes.
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::PreferDark);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Mark-Hulk")
        .default_width(1100)
        .default_height(720)
        .width_request(680)
        .height_request(480)
        .build();

    // --- Editor (GtkSourceView) -------------------------------------------
    let source_buffer = sourceview5::Buffer::builder().build();
    if let Some(lang) = sourceview5::LanguageManager::default().language("markdown") {
        source_buffer.set_language(Some(&lang));
    }
    if let Some(scheme) = sourceview5::StyleSchemeManager::default().scheme("Adwaita-dark") {
        source_buffer.set_style_scheme(Some(&scheme));
    }
    let source_view = sourceview5::View::with_buffer(&source_buffer);
    source_view.set_monospace(true);
    source_view.set_show_line_numbers(true);
    source_view.set_highlight_current_line(true);
    source_view.set_wrap_mode(gtk::WrapMode::WordChar);
    source_view.set_top_margin(8);
    source_view.set_bottom_margin(8);
    source_view.set_left_margin(8);
    source_view.set_right_margin(8);

    let editor_scroll = gtk::ScrolledWindow::builder()
        .child(&source_view)
        .hexpand(true)
        .vexpand(true)
        .build();

    // --- Preview (Pango markup in a label) --------------------------------
    let preview_label = gtk::Label::builder()
        .use_markup(true)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .xalign(0.0)
        .yalign(0.0)
        .selectable(true)
        .valign(gtk::Align::Start)
        .halign(gtk::Align::Fill)
        .build();
    preview_label.set_markup("<i>Open a folder or a file to begin.</i>");

    let preview_holder = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(20)
        .margin_end(20)
        .build();
    preview_holder.append(&preview_label);

    let preview_scroll = gtk::ScrolledWindow::builder()
        .child(&preview_holder)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .vexpand(true)
        .build();

    // --- Sidebar (file tree) ----------------------------------------------
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
        .build();

    // --- Content split (editor | preview) ---------------------------------
    let content_paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&editor_scroll)
        .end_child(&preview_scroll)
        .position(540)
        .build();

    // --- Outer split (sidebar | content) ----------------------------------
    let main_paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&sidebar_scroll)
        .end_child(&content_paned)
        .position(240)
        .resize_start_child(false)
        .build();

    // --- Header bar -------------------------------------------------------
    let header = adw::HeaderBar::new();

    let open_folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
    open_folder_btn.set_tooltip_text(Some("Open folder"));
    let open_file_btn = gtk::Button::from_icon_name("document-open-symbolic");
    open_file_btn.set_tooltip_text(Some("Open file"));
    header.pack_start(&open_folder_btn);
    header.pack_start(&open_file_btn);

    let save_btn = gtk::Button::from_icon_name("document-save-symbolic");
    save_btn.set_tooltip_text(Some("Save (Ctrl+S)"));
    header.pack_end(&save_btn);

    // View-mode toggle group.
    let preview_btn = gtk::ToggleButton::with_label("Preview");
    let split_btn = gtk::ToggleButton::with_label("Split");
    let edit_btn = gtk::ToggleButton::with_label("Edit");
    split_btn.set_group(Some(&preview_btn));
    edit_btn.set_group(Some(&preview_btn));
    split_btn.set_active(true);
    let mode_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .css_classes(vec!["linked".to_string()])
        .build();
    mode_box.append(&preview_btn);
    mode_box.append(&split_btn);
    mode_box.append(&edit_btn);
    header.pack_end(&mode_box);

    // --- Assemble ---------------------------------------------------------
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&main_paned));
    window.set_content(Some(&toolbar_view));

    let ui = Rc::new(Ui {
        window: window.clone(),
        sidebar_list,
        source_buffer: source_buffer.clone(),
        preview_label,
        editor_scroll,
        preview_scroll,
        current_path: RefCell::new(None),
        root: RefCell::new(None),
    });

    // --- Wire up signals --------------------------------------------------
    {
        let ui = ui.clone();
        source_buffer.connect_changed(move |_| refresh_preview(&ui));
    }
    {
        let ui = ui.clone();
        open_folder_btn.connect_clicked(move |_| open_folder(&ui));
    }
    {
        let ui = ui.clone();
        open_file_btn.connect_clicked(move |_| open_file(&ui));
    }
    {
        let ui = ui.clone();
        save_btn.connect_clicked(move |_| save(&ui));
    }
    {
        let ui = ui.clone();
        preview_btn.connect_toggled(move |b| {
            if b.is_active() {
                set_mode(&ui, Mode::Preview);
            }
        });
    }
    {
        let ui = ui.clone();
        split_btn.connect_toggled(move |b| {
            if b.is_active() {
                set_mode(&ui, Mode::Split);
            }
        });
    }
    {
        let ui = ui.clone();
        edit_btn.connect_toggled(move |b| {
            if b.is_active() {
                set_mode(&ui, Mode::Edit);
            }
        });
    }

    // Ctrl+S to save.
    let save_action = gio::SimpleAction::new("save", None);
    {
        let ui = ui.clone();
        save_action.connect_activate(move |_, _| save(&ui));
    }
    window.add_action(&save_action);
    app.set_accels_for_action("win.save", &["<Primary>s"]);

    set_mode(&ui, Mode::Split);
    window.present();
}

fn refresh_preview(ui: &Rc<Ui>) {
    let buf = &ui.source_buffer;
    let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
    let markup = markdown::to_pango_markup(&text);
    ui.preview_label.set_markup(&markup);
}

fn set_mode(ui: &Rc<Ui>, mode: Mode) {
    let (edit, preview) = match mode {
        Mode::Preview => (false, true),
        Mode::Split => (true, true),
        Mode::Edit => (true, false),
    };
    ui.editor_scroll.set_visible(edit);
    ui.preview_scroll.set_visible(preview);
}

fn load_file(ui: &Rc<Ui>, path: &Path) {
    match fs::read_file(path) {
        Ok(text) => {
            ui.source_buffer.set_text(&text);
            *ui.current_path.borrow_mut() = Some(path.to_path_buf());
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.window.set_title(Some(&format!("{name} — Mark-Hulk")));
        }
        Err(e) => eprintln!("Failed to read {}: {e}", path.display()),
    }
}

fn save(ui: &Rc<Ui>) {
    let path = ui.current_path.borrow().clone();
    if let Some(path) = path {
        let buf = &ui.source_buffer;
        let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
        if let Err(e) = fs::write_file(&path, &text) {
            eprintln!("Failed to save {}: {e}", path.display());
        }
    }
}

fn open_folder(ui: &Rc<Ui>) {
    let dialog = gtk::FileDialog::builder().title("Open folder").build();
    let ui = ui.clone();
    dialog.select_folder(
        Some(&ui.window.clone()),
        gio::Cancellable::NONE,
        move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() {
                    populate_tree(&ui, &path);
                }
            }
        },
    );
}

fn open_file(ui: &Rc<Ui>) {
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
    let ui = ui.clone();
    dialog.open(Some(&ui.window.clone()), gio::Cancellable::NONE, move |res| {
        if let Ok(file) = res {
            if let Some(path) = file.path() {
                load_file(&ui, &path);
            }
        }
    });
}

fn populate_tree(ui: &Rc<Ui>, root: &Path) {
    *ui.root.borrow_mut() = Some(root.to_path_buf());

    while let Some(child) = ui.sidebar_list.first_child() {
        ui.sidebar_list.remove(&child);
    }

    if let Some(tree) = fs::build_tree(root) {
        for child in &tree.children {
            let w = tree_widget(ui, child);
            ui.sidebar_list.append(&w);
        }
    }
}

fn tree_widget(ui: &Rc<Ui>, node: &fs::TreeNode) -> gtk::Widget {
    if node.is_dir {
        let expander = gtk::Expander::new(Some(&node.name));
        let inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(2)
            .margin_start(12)
            .build();
        for child in &node.children {
            inner.append(&tree_widget(ui, child));
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
        let ui = ui.clone();
        button.connect_clicked(move |_| load_file(&ui, &path));
        button.upcast()
    }
}
