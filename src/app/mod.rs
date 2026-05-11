use gtk::prelude::*;

pub fn build(startup: Option<crate::cli::StartupAction>) -> gtk::Application {
    let app = gtk::Application::builder()
        .application_id("land.hypr.Hyprscreen")
        .build();

    app.connect_activate(move |app| {
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(true);
        }
        load_css();
        let window = crate::ui::main_window::build(app, startup);
        window.present();
        crate::hyprland::float_window_once();
    });

    app
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CSS);
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("display unavailable for CSS provider"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

const CSS: &str = r#"
window {
    background: #16181E;
    color: #E8E9EC;
    font-family: Cantarell, "Inter Tight", "Segoe UI", sans-serif;
}

window.hs-rec-indicator,
window.hs-rec-indicator > * {
    background: transparent;
}

button {
    background-image: none;
    outline: none;
    box-shadow: none;
    min-height: 0;
    padding: 0;
}


.hs-body {
    padding: 16px 18px 14px 18px;
}

.hs-seg {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 3px;
}

.hs-seg > button {
    border: none;
    background: transparent;
    border-radius: 6px;
    padding: 7px 0;
    min-height: 0;
}

.hs-seg > button:hover {
    color: #E8E9EC;
}

.hs-seg > button:checked {
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 1px 0 rgba(255,255,255,0.05) inset, 0 1px 2px rgba(0,0,0,0.25);
}

.hs-seg-label {
    color: #8B8D95;
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
}

.hs-seg > button:hover .hs-seg-label,
.hs-seg > button:checked .hs-seg-label {
    color: #E8E9EC;
}

.hs-tbtn {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 0 !important;
    min-height: 0 !important;
}

.hs-tbtn:hover {
    background: rgba(255,255,255,0.05);
    border-color: rgba(255,255,255,0.14);
}

.hs-tbtn:active {
    transform: translateY(1px);
}

.hs-tbtn:checked {
    border-color: rgba(255,255,255,0.22);
    background: rgba(255,255,255,0.07);
    box-shadow: 0 0 0 1px rgba(255,255,255,0.04) inset;
}

.hs-tbtn.mode-rec:checked {
    border-color: rgba(229,72,77,0.55);
    background: rgba(229,72,77,0.16);
}

.hs-tbtn.mode-shot:checked {
    border-color: rgba(229,236,245,0.45);
    background: rgba(229,236,245,0.16);
}

.hs-tbtn-label {
    color: #8B8D95;
    font-size: 12px;
    font-weight: 500;
    line-height: 1;
}

.hs-tbtn-icon {
    opacity: 0.5;
}

.hs-tbtn:hover .hs-tbtn-label,
.hs-tbtn:checked .hs-tbtn-label {
    color: #E8E9EC;
}

.hs-tbtn:hover .hs-tbtn-icon,
.hs-tbtn:checked .hs-tbtn-icon {
    opacity: 1;
}

.hs-primary {
    border: none;
    border-radius: 8px;
    padding: 13px 16px;
    min-height: 0;
}

.hs-primary.mode-shot {
    background: #E5ECF5;
    color: #0E1116;
}

.hs-primary.mode-rec {
    background: #E5484D;
    color: #FFFFFF;
}

.hs-primary:hover {
    filter: brightness(1.06);
}

.hs-primary:active {
    transform: translateY(1px);
    filter: brightness(0.94);
}

.hs-primary:disabled {
    opacity: 0.45;
}

.hs-primary-label {
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: 0.01em;
}

.hs-primary-pulse {
    min-width: 8px;
    min-height: 8px;
    background: #FFFFFF;
    border-radius: 999px;
    box-shadow: 0 0 0 0 rgba(255,255,255,0.5);
    animation: hsPulse 1.4s ease-out infinite;
}


@keyframes hsPulse {
    0% { box-shadow: 0 0 0 0 rgba(255,255,255,0.5); }
    70% { box-shadow: 0 0 0 8px rgba(255,255,255,0); }
    100% { box-shadow: 0 0 0 0 rgba(255,255,255,0); }
}

.hs-optrow {
    padding: 4px 2px 0 2px;
}

.hs-opt-label {
    color: #8B8D95;
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.01em;
}

.hs-opt-hint {
    color: #5C5E66;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.04em;
}

.hs-opt-dot {
    min-width: 6px;
    min-height: 6px;
    background: #5C5E66;
    border-radius: 999px;
}

.hs-optrow.is-on .hs-opt-dot {
    background: #E5484D;
    box-shadow: 0 0 6px 1px rgba(229,72,77,0.45);
}

.hs-switch {
    min-width: 34px;
    min-height: 18px;
    padding: 2px;
    border-radius: 999px;
    border: 1px solid rgba(255,255,255,0.08);
    background: rgba(255,255,255,0.07);
    outline: none;
    box-shadow: none;
}

.hs-switch:checked {
    background: rgba(229,72,77,0.16);
    border-color: rgba(229,72,77,0.55);
}

.hs-switch slider {
    min-width: 12px;
    min-height: 12px;
    border-radius: 999px;
    background: rgba(255,255,255,0.55);
    border: none;
    box-shadow: none;
}

.hs-switch:checked slider {
    background: #E5484D;
}

.hs-status {
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    color: #5C5E66;
    min-height: 14px;
}

.hs-status.err { color: #F0848A; }
.hs-status.ok { color: #7FCB9B; }
.hs-status.live { color: #E5ECF5; }

.hs-meta {
    color: #8B8D95;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.03em;
}

.hs-preview-frame {
    background: #0C0D11;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
}

.hs-abtn {
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 7px;
    padding: 9px 4px 8px 4px;
    min-height: 0;
}

.hs-abtn:hover {
    background: rgba(255,255,255,0.06);
    border-color: rgba(255,255,255,0.14);
}

.hs-abtn:active {
    transform: translateY(1px);
}

.hs-abtn:disabled {
    opacity: 0.38;
}

.hs-abtn-label {
    color: #E8E9EC;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: 1;
}

.hs-abtn.is-primary {
    background: rgba(229,236,245,0.10);
    border-color: rgba(229,236,245,0.30);
}

.hs-abtn.is-primary.mode-rec {
    background: rgba(229,72,77,0.16);
    border-color: rgba(229,72,77,0.42);
}

.hs-hud {
    background: rgba(18, 19, 24, 0.82);
    border: 1px solid rgba(255,255,255,0.10);
    padding: 8px 10px 8px 14px;
}

.hs-hud-dot {
    background: #E5484D;
    border-radius: 999px;
    min-width: 9px;
    min-height: 9px;
    animation: hudPulse 1.6s ease-out infinite;
}

@keyframes hudPulse {
    0% { box-shadow: 0 0 0 0 rgba(229,72,77,0.55); }
    70% { box-shadow: 0 0 0 7px rgba(229,72,77,0); }
    100% { box-shadow: 0 0 0 0 rgba(229,72,77,0); }
}

.hs-hud-rec {
    color: #8B8D95;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.14em;
}

.hs-hud-timer {
    color: #E8E9EC;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.02em;
}

.hs-hud-sep {
    background: rgba(255,255,255,0.08);
    min-width: 1px;
    min-height: 16px;
}

.hs-hud-stop {
    border: none;
    background: #E5484D;
    color: #FFFFFF;
    border-radius: 999px;
    padding: 7px 12px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    min-height: 0;
}

.hs-hud-stop:hover {
    filter: brightness(1.08);
}

window.hs-mon-id {
    background: #0E1116;
    border: 2px solid rgba(255, 255, 255, 0.10);
    border-radius: 14px;
}

.hs-mon-id-label {
    color: #E8E9EC;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 32px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 18px 24px;
}

label.hs-rec-flash {
    color: #E5484D;
    font-size: 14px;
    font-weight: 700;
    background: transparent;
}


/* ── Tinted target-button labels in checked state ── */
.hs-tbtn.mode-rec:checked  .hs-tbtn-label { color: #FBD5D6; }
.hs-tbtn.mode-shot:checked .hs-tbtn-label { color: #E5ECF5; }

/* ── Primary action-button (Save) accent label ── */
.hs-abtn.is-primary          .hs-abtn-label { color: #E5ECF5; }
.hs-abtn.is-primary.mode-rec .hs-abtn-label { color: #FBD5D6; }
"#;
