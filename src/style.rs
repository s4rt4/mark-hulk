//! App theming. Three themes — "Forest Sage" (default), "Dark Emerald", and the
//! light "Light Sage" — each pairing a libadwaita palette (CSS named-color
//! overrides) with a matching GtkSourceView editor scheme. The app is called
//! Mark-*Hulk*: green is the point, light or dark.

pub const FOREST: &str = "forest";
pub const EMERALD: &str = "emerald";
pub const LIGHT: &str = "light";

/// Whether `theme` is a light theme. Drives the adwaita color-scheme and the
/// syntect code-highlight palette, which both differ light vs dark.
pub fn is_light(theme: &str) -> bool {
    theme == LIGHT
}

const FOREST_CSS: &str = r#"
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
.mh-preview a { color: #95D1AF; }
"#;

const EMERALD_CSS: &str = r#"
@define-color window_bg_color   #0A0C0A;
@define-color window_fg_color   #D4E6DC;
@define-color view_bg_color     #0B0F0D;
@define-color view_fg_color     #D4E6DC;
@define-color headerbar_bg_color #0C100E;
@define-color headerbar_fg_color #D4E6DC;
@define-color card_bg_color     #0E1411;
@define-color popover_bg_color  #0E1411;
@define-color popover_fg_color  #D4E6DC;
@define-color dialog_bg_color   #0B0F0D;
@define-color dialog_fg_color   #D4E6DC;
@define-color sidebar_bg_color  #080A08;
@define-color sidebar_fg_color  #D4E6DC;
@define-color accent_bg_color   #2ECC71;
@define-color accent_fg_color   #05130B;
@define-color accent_color      #46D886;

.mh-sidebar { background-color: @sidebar_bg_color; }
.mh-sidebar button { background: transparent; border-radius: 6px; }
.mh-sidebar button:hover { background-color: alpha(#2ECC71, 0.14); }
.mh-preview a { color: #46D886; }
"#;

const LIGHT_CSS: &str = r#"
@define-color window_bg_color   #F2F8F4;
@define-color window_fg_color   #143025;
@define-color view_bg_color     #FFFFFF;
@define-color view_fg_color     #143025;
@define-color headerbar_bg_color #E8F1EB;
@define-color headerbar_fg_color #143025;
@define-color card_bg_color     #FFFFFF;
@define-color popover_bg_color  #FFFFFF;
@define-color popover_fg_color  #143025;
@define-color dialog_bg_color   #FFFFFF;
@define-color dialog_fg_color   #143025;
@define-color sidebar_bg_color  #E8F1EB;
@define-color sidebar_fg_color  #143025;
@define-color accent_bg_color   #1E9E59;
@define-color accent_fg_color   #FFFFFF;
@define-color accent_color      #1B7A45;

.mh-sidebar { background-color: @sidebar_bg_color; }
.mh-sidebar button { background: transparent; border-radius: 6px; }
.mh-sidebar button:hover { background-color: alpha(#1E9E59, 0.12); }
.mh-preview a { color: #1B7A45; }
"#;

fn css_for(theme: &str) -> &'static str {
    match theme {
        EMERALD => EMERALD_CSS,
        LIGHT => LIGHT_CSS,
        _ => FOREST_CSS,
    }
}

/// Create the shared CSS provider and attach it to the default display.
pub fn make_provider() -> gtk::CssProvider {
    let provider = gtk::CssProvider::new();
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    provider
}

/// Swap the provider's palette to `theme`.
pub fn apply_css(provider: &gtk::CssProvider, theme: &str) {
    provider.load_from_data(css_for(theme));
}

/// Write both bundled editor schemes into the user's data dir and register them.
pub fn install_schemes() {
    let dir = gtk::glib::user_data_dir().join("mark-hulk").join("styles");
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(
            dir.join("mark-hulk-forest.xml"),
            include_str!("../data/mark-hulk-forest.xml"),
        );
        let _ = std::fs::write(
            dir.join("mark-hulk-emerald.xml"),
            include_str!("../data/mark-hulk-emerald.xml"),
        );
        let _ = std::fs::write(
            dir.join("mark-hulk-light.xml"),
            include_str!("../data/mark-hulk-light.xml"),
        );
        if let Some(path) = dir.to_str() {
            sourceview5::StyleSchemeManager::default().append_search_path(path);
        }
    }
}

fn theme_config_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir().join("mark-hulk").join("theme")
}

/// Read the saved theme name, defaulting to Forest Sage.
pub fn load_saved_theme() -> String {
    match std::fs::read_to_string(theme_config_path()) {
        Ok(s) if s.trim() == EMERALD => EMERALD.to_string(),
        Ok(s) if s.trim() == LIGHT => LIGHT.to_string(),
        _ => FOREST.to_string(),
    }
}

/// Persist the chosen theme so it survives a restart.
pub fn save_theme(theme: &str) {
    let path = theme_config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, theme);
}

/// The editor scheme for `theme`. Light falls back to plain Adwaita; the dark
/// themes fall back to forest, then Adwaita-dark.
pub fn scheme_for(theme: &str) -> Option<sourceview5::StyleScheme> {
    let mgr = sourceview5::StyleSchemeManager::default();
    let id = match theme {
        EMERALD => "mark-hulk-emerald",
        LIGHT => "mark-hulk-light",
        _ => "mark-hulk-forest",
    };
    let fallback = if is_light(theme) {
        "Adwaita"
    } else {
        "Adwaita-dark"
    };
    mgr.scheme(id)
        .or_else(|| mgr.scheme("mark-hulk-forest"))
        .or_else(|| mgr.scheme(fallback))
}
