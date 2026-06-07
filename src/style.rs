//! App theming: a green "Forest Sage" palette applied over libadwaita, plus the
//! matching GtkSourceView editor scheme. The app is called Mark-*Hulk* — green
//! is the whole point.

/// libadwaita named-color overrides — recolors window, header bar, sidebar,
/// buttons, accents, popovers and dialogs in one shot.
const CSS: &str = r#"
@define-color window_bg_color   #0B231A;
@define-color window_fg_color   #D6ECE0;
@define-color view_bg_color     #0C2A20;
@define-color view_fg_color     #D6ECE0;
@define-color headerbar_bg_color #0D2820;
@define-color headerbar_fg_color #D6ECE0;
@define-color card_bg_color     #0E2A20;
@define-color popover_bg_color  #0E2A20;
@define-color popover_fg_color  #D6ECE0;
@define-color dialog_bg_color   #0C2A20;
@define-color dialog_fg_color   #D6ECE0;
@define-color sidebar_bg_color  #0A2017;
@define-color sidebar_fg_color  #D6ECE0;
@define-color accent_bg_color   #2ECC71;
@define-color accent_fg_color   #06160F;
@define-color accent_color      #95D1AF;

.mh-sidebar { background-color: @sidebar_bg_color; }
.mh-sidebar button { background: transparent; border-radius: 6px; }
.mh-sidebar button:hover { background-color: alpha(#2ECC71, 0.12); }

/* Pango links in the preview label follow the sage accent. */
.mh-preview a { color: #95D1AF; }
"#;

/// Load the global CSS into the default display.
pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Install the bundled editor scheme into the user's data dir and return it.
/// Falls back to "Adwaita-dark" if anything goes wrong.
pub fn editor_scheme() -> Option<sourceview5::StyleScheme> {
    let mgr = sourceview5::StyleSchemeManager::default();

    let dir = gtk::glib::user_data_dir().join("mark-hulk").join("styles");
    if std::fs::create_dir_all(&dir).is_ok() {
        let file = dir.join("mark-hulk-forest.xml");
        let _ = std::fs::write(&file, include_str!("../data/mark-hulk-forest.xml"));
        if let Some(path) = dir.to_str() {
            mgr.append_search_path(path);
        }
    }

    mgr.scheme("mark-hulk-forest")
        .or_else(|| mgr.scheme("Adwaita-dark"))
}
