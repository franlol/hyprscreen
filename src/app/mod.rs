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
        if startup.is_none() {
            window.present();
        }
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

/* v2 design tokens (docs/adr/0011). GTK CSS has no var(); values are literal.
   --d-bg          rgba(19,20,26,0.72)   translucent surface (glass)
   --d-bg-solid    #14151B               solid surface fallback
   --d-elevated    rgba(30,32,40,0.86)   popovers / toasts
   --d-fill        rgba(255,255,255,0.05)
   --d-fill-hover  rgba(255,255,255,0.09)
   --d-border      rgba(255,255,255,0.10)
   --d-border-strong rgba(255,255,255,0.18)
   --d-text  #EDEEF2   --d-muted #9A9CA6   --d-dim #62646E
   shot accent: #5EE6D0  soft rgba(94,230,208,0.16)  fg #06231F
   rec  accent: #FF5D5D  soft rgba(255,93,93,0.16)   fg #2A0808
   saved/ok: #6FD79E */
const CSS: &str = r#"
window {
    background: #14151B;
    color: #EDEEF2;
    font-family: Cantarell, "Inter Tight", "Segoe UI", sans-serif;
}

window.hs-glass {
    background: rgba(19, 20, 26, 0.72);
}

window.hs-rec-indicator,
window.hs-rec-indicator > * {
    background: transparent;
}

tooltip {
    background: #14151B;
    color: #EDEEF2;
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 6px;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
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
    background: rgba(0, 0, 0, 0.22);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 10px;
    padding: 3px;
}

.hs-seg > button {
    border: none;
    background: transparent;
    border-radius: 7px;
    padding: 7px 0;
    min-height: 0;
    transition: all 130ms;
}

.hs-seg > button:hover {
    color: #EDEEF2;
}

.hs-seg > button:checked {
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 0 0 1px rgba(255,255,255,0.18) inset;
}

.hs-seg-label {
    color: #9A9CA6;
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
}

.hs-seg > button:hover .hs-seg-label,
.hs-seg > button:checked .hs-seg-label {
    color: #EDEEF2;
}

.hs-tbtn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 11px;
    padding: 0;
    min-height: 0;
    transition: all 130ms;
}

.hs-tbtn:hover {
    background: rgba(255,255,255,0.09);
    border-color: rgba(255,255,255,0.14);
}

.hs-tbtn:active {
    transform: translateY(1px);
}

.hs-tbtn:checked {
    border-color: rgba(255,255,255,0.18);
    background: rgba(255,255,255,0.05);
}

.hs-tbtn.mode-rec:checked {
    border-color: rgba(255,93,93,0.55);
    background: rgba(255,93,93,0.16);
}

.hs-tbtn.mode-shot:checked {
    border-color: rgba(94,230,208,0.45);
    background: rgba(94,230,208,0.16);
}

.hs-tbtn-label {
    color: #9A9CA6;
    font-size: 12px;
    font-weight: 500;
    line-height: 1;
}

.hs-tbtn-icon {
    opacity: 0.55;
}

.hs-tbtn:hover .hs-tbtn-label,
.hs-tbtn:checked .hs-tbtn-label {
    color: #EDEEF2;
}

.hs-tbtn:hover .hs-tbtn-icon,
.hs-tbtn:checked .hs-tbtn-icon {
    opacity: 1;
}

.hs-primary {
    border: none;
    border-radius: 13px;
    padding: 13px 16px;
    min-height: 0;
    transition: all 120ms;
}

.hs-primary.mode-shot {
    background: #5EE6D0;
    color: #06231F;
}

.hs-primary.mode-rec {
    background: #FF5D5D;
    color: #2A0808;
}

.hs-primary:hover {
    filter: brightness(1.07);
}

.hs-primary:active {
    transform: translateY(1px);
    filter: brightness(0.95);
}

.hs-primary:disabled {
    opacity: 0.45;
}

.hs-primary-label {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.01em;
}

.hs-primary-pulse {
    min-width: 8px;
    min-height: 8px;
    background: #2A0808;
    border-radius: 999px;
    box-shadow: 0 0 0 0 rgba(42,8,8,0.5);
    animation: hsPulse 1.4s ease-out infinite;
}


@keyframes hsPulse {
    0% { box-shadow: 0 0 0 0 rgba(42,8,8,0.45); }
    70% { box-shadow: 0 0 0 8px rgba(42,8,8,0); }
    100% { box-shadow: 0 0 0 0 rgba(42,8,8,0); }
}

.hs-optrow {
    padding: 4px 2px 0 2px;
}

.hs-opt-label {
    color: #9A9CA6;
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.01em;
}

.hs-opt-hint {
    color: #62646E;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.04em;
}

.hs-opt-dot {
    min-width: 6px;
    min-height: 6px;
    background: #62646E;
    border-radius: 999px;
}

.hs-optrow.is-on .hs-opt-dot {
    background: #FF5D5D;
    box-shadow: 0 0 6px 1px rgba(255,93,93,0.45);
}

.hs-switch {
    min-width: 38px;
    min-height: 22px;
    padding: 1px;
    border-radius: 999px;
    border: 1px solid rgba(255,255,255,0.10);
    background: rgba(255,255,255,0.08);
    outline: none;
    box-shadow: none;
    transition: background 140ms, border-color 140ms;
}

.hs-switch:checked {
    background: rgba(255,93,93,0.16);
    border-color: #FF5D5D;
}

.hs-switch slider {
    min-width: 18px;
    min-height: 18px;
    border-radius: 999px;
    background: #C7C8CC;
    border: none;
    box-shadow: 0 1px 2px rgba(0,0,0,0.4);
}

.hs-switch:checked slider {
    background: #FF5D5D;
}

.hs-status {
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    color: #62646E;
    min-height: 14px;
}

.hs-status.err { color: #FF5D5D; }
.hs-status.ok { color: #6FD79E; }
.hs-status.live { color: #5EE6D0; }

.hs-meta {
    color: #9A9CA6;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.03em;
}

.hs-preview-frame {
    background: #0C0D11;
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 10px;
}

.hs-abtn {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 9px;
    padding: 9px 4px 8px 4px;
    min-height: 0;
    transition: all 120ms;
}

.hs-abtn:hover {
    background: rgba(255,255,255,0.09);
    border-color: rgba(255,255,255,0.14);
}

.hs-abtn:active {
    transform: translateY(1px);
}

.hs-abtn:disabled {
    opacity: 0.38;
}

.hs-abtn-label {
    color: #EDEEF2;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: 1;
}

.hs-abtn.is-primary {
    background: rgba(94,230,208,0.16);
    border-color: rgba(94,230,208,0.42);
}

.hs-abtn.is-primary.mode-rec {
    background: rgba(255,93,93,0.16);
    border-color: rgba(255,93,93,0.42);
}

.hs-hud {
    background: rgba(19, 20, 26, 0.72);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 999px;
    padding: 8px 10px 8px 14px;
}

.hs-hud-dot {
    background: #FF5D5D;
    border-radius: 999px;
    min-width: 9px;
    min-height: 9px;
    animation: hudPulse 1.6s ease-out infinite;
}

@keyframes hudPulse {
    0% { box-shadow: 0 0 0 0 rgba(255,93,93,0.55); }
    70% { box-shadow: 0 0 0 7px rgba(255,93,93,0); }
    100% { box-shadow: 0 0 0 0 rgba(255,93,93,0); }
}

.hs-hud-rec {
    color: #9A9CA6;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.14em;
}

.hs-hud-timer {
    color: #EDEEF2;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
}

.hs-hud-sep {
    background: rgba(255,255,255,0.10);
    min-width: 1px;
    min-height: 16px;
}

.hs-hud-stop {
    border: none;
    background: #FF5D5D;
    color: #FFFFFF;
    border-radius: 999px;
    padding: 7px 13px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    min-height: 0;
}

.hs-hud-stop:hover {
    filter: brightness(1.08);
}

window.hs-mon-id {
    background: #14151B;
    border: 2px solid rgba(255, 255, 255, 0.10);
    border-radius: 14px;
}

.hs-mon-id-label {
    color: #EDEEF2;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 32px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 18px 24px;
}

label.hs-rec-flash {
    color: #FF5D5D;
    font-size: 14px;
    font-weight: 700;
    background: transparent;
}


/* ── Tinted target-button labels in checked state ── */
.hs-tbtn.mode-rec:checked  .hs-tbtn-label { color: #FF5D5D; }
.hs-tbtn.mode-shot:checked .hs-tbtn-label { color: #5EE6D0; }

/* ── Primary action-button (Save) accent label ── */
.hs-abtn.is-primary          .hs-abtn-label { color: #5EE6D0; }
.hs-abtn.is-primary.mode-rec .hs-abtn-label { color: #FF5D5D; }
"#;
