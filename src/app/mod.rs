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


/* ═══════════════ Capture Dock (ADR-0012) ═══════════════ */

.hs-dock {
    padding: 8px;
}

.hs-dseg {
    background: rgba(0, 0, 0, 0.22);
    border-radius: 12px;
    padding: 3px;
}

.hs-dseg-btn {
    border: none;
    background: transparent;
    border-radius: 9px;
    padding: 8px 11px;
    min-height: 0;
    transition: all 130ms;
}

.hs-dseg-label {
    color: #9A9CA6;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.01em;
    line-height: 1;
}

.hs-dseg-btn:hover .hs-dseg-label {
    color: #EDEEF2;
}

.hs-dseg-btn.hs-seg-shot:checked {
    background: rgba(94, 230, 208, 0.16);
    box-shadow: 0 0 0 1px rgba(94, 230, 208, 0.25) inset;
}
.hs-dseg-btn.hs-seg-shot:checked .hs-dseg-label { color: #5EE6D0; }

.hs-dseg-btn.hs-seg-rec:checked {
    background: rgba(255, 93, 93, 0.16);
    box-shadow: 0 0 0 1px rgba(255, 93, 93, 0.28) inset;
}
.hs-dseg-btn.hs-seg-rec:checked .hs-dseg-label { color: #FF5D5D; }

.hs-ddiv {
    background: rgba(255, 255, 255, 0.10);
    min-width: 1px;
}

.hs-dico {
    border: none;
    background: transparent;
    border-radius: 11px;
    padding: 0;
    min-height: 0;
    transition: all 130ms;
}

.hs-dico:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-dico:active {
    transform: translateY(1px);
}

.hs-dico:checked {
    background: rgba(255, 255, 255, 0.05);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.18) inset;
}

.hs-dico:disabled {
    opacity: 0.35;
}

.hs-dico-icon { opacity: 0.6; }
.hs-dico:hover .hs-dico-icon,
.hs-dico:checked .hs-dico-icon { opacity: 1; }

.hs-dico-dot {
    min-width: 4px;
    min-height: 4px;
    border-radius: 999px;
    background: #5EE6D0;
}
window.hs-mode-rec .hs-dico-dot { background: #FF5D5D; }

.hs-dchip {
    border: none;
    background: transparent;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 11px;
    transition: all 130ms;
}

.hs-dchip:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-dchip-label {
    color: #9A9CA6;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    line-height: 1;
}

.hs-dchip:hover .hs-dchip-label { color: #EDEEF2; }
window.hs-mode-shot .hs-dchip.on .hs-dchip-label { color: #5EE6D0; }
window.hs-mode-rec  .hs-dchip.on .hs-dchip-label { color: #FF5D5D; }

.hs-dmore > button {
    border: none;
    background: transparent;
    min-width: 40px;
    min-height: 40px;
    border-radius: 11px;
    padding: 0;
    transition: all 130ms;
}

.hs-dmore > button:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-dmore > button:checked {
    background: rgba(255, 255, 255, 0.05);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.18) inset;
}

.hs-dfire {
    min-height: 44px;
    padding: 0 20px 0 16px;
    border-radius: 13px;
    margin-left: 4px;
}

.hs-fire-kbd {
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10px;
    font-weight: 600;
    padding: 3px 6px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.18);
    opacity: 0.7;
}

.hs-rec-ring {
    border-radius: 999px;
    border: 3px solid #2A0808;
}

/* ── Quick settings popover ── */
popover.hs-qpop > contents {
    background: rgba(30, 32, 40, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 16px;
    padding: 14px;
    box-shadow: 0 20px 50px -12px rgba(0, 0, 0, 0.6);
}

popover.hs-qpop > arrow {
    background: rgba(30, 32, 40, 0.96);
}

.hs-qlabel {
    color: #9A9CA6;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
}

.hs-qseg {
    background: rgba(0, 0, 0, 0.22);
    border-radius: 10px;
    padding: 3px;
}

.hs-qseg-btn {
    border: none;
    background: transparent;
    border-radius: 7px;
    padding: 8px 0;
    min-height: 0;
    color: #9A9CA6;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    transition: all 120ms;
}

.hs-qseg-btn:hover { color: #EDEEF2; }

popover.hs-mode-shot .hs-qseg-btn:checked {
    background: rgba(94, 230, 208, 0.16);
    color: #5EE6D0;
    box-shadow: 0 0 0 1px #5EE6D0 inset;
}

popover.hs-mode-rec .hs-qseg-btn:checked {
    background: rgba(255, 93, 93, 0.16);
    color: #FF5D5D;
    box-shadow: 0 0 0 1px #FF5D5D inset;
}

.hs-qtoggle-name {
    color: #EDEEF2;
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1;
}

.hs-qtoggle-sub {
    color: #62646E;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: 1;
}

popover.hs-mode-shot .hs-switch:checked {
    background: rgba(94, 230, 208, 0.16);
    border-color: #5EE6D0;
}
popover.hs-mode-shot .hs-switch:checked slider { background: #5EE6D0; }


/* ═══════════════ Corner thumbnail (ADR-0013) ═══════════════ */

window.hs-thumb-window {
    background: rgba(19, 20, 26, 0.72);
}

.hs-thumb-preview {
    background: #0C0D11;
}

.hs-thumb-badge {
    background: rgba(0, 0, 0, 0.4);
    border-radius: 7px;
    padding: 4px 8px;
}

.hs-thumb-badge-label {
    color: #5EE6D0;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    line-height: 1;
}

.hs-thumb-bdot {
    background: #5EE6D0;
    border-radius: 999px;
}

.hs-thumb-badge.rec .hs-thumb-badge-label { color: #FF5D5D; }
.hs-thumb-badge.rec .hs-thumb-bdot { background: #FF5D5D; }

.hs-thumb-close {
    border: none;
    background: rgba(0, 0, 0, 0.4);
    border-radius: 7px;
    padding: 0;
    min-height: 0;
}

.hs-thumb-close:hover {
    background: rgba(0, 0, 0, 0.6);
}

.hs-thumb-play {
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: rgba(0, 0, 0, 0.45);
    border-radius: 999px;
    padding: 0;
    min-height: 0;
}

.hs-thumb-play:hover {
    background: rgba(0, 0, 0, 0.65);
}

.hs-thumb-bar {
    border-top: 1px solid rgba(255, 255, 255, 0.10);
    padding: 6px 0 4px 0;
}

.hs-tbtn2 {
    border: none;
    background: transparent;
    border-radius: 9px;
    padding: 0;
    min-height: 0;
    transition: all 120ms;
}

.hs-tbtn2:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-tbtn2:active {
    transform: translateY(1px);
}

.hs-tbtn2:disabled {
    opacity: 0.35;
}

.hs-tbtn2.accent:hover {
    background: rgba(94, 230, 208, 0.16);
}

.hs-tbtn2.accent.rec:hover {
    background: rgba(255, 93, 93, 0.16);
}

.hs-thumb-meta {
    color: #62646E;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.03em;
    line-height: 1;
}


/* ═══════════════ Toasts (ADR-0014) ═══════════════ */

window.hs-toast-window {
    background: rgba(30, 32, 40, 0.86);
}

window.hs-toast-window-solid {
    background: #1E2028;
}

.hs-toast {
    padding: 11px 15px;
}

.hs-toast-ico {
    border-radius: 9px;
}

.hs-toast-ico.ok {
    background: rgba(111, 215, 158, 0.16);
}

.hs-toast-ico.err {
    background: rgba(255, 93, 93, 0.16);
}

.hs-toast-title {
    color: #EDEEF2;
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.2;
}

.hs-toast-sub {
    color: #9A9CA6;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: 1.3;
}

.hs-toast-action {
    border: none;
    background: rgba(255, 255, 255, 0.05);
    color: #EDEEF2;
    border-radius: 8px;
    padding: 6px 11px;
    min-height: 0;
    margin-left: 6px;
    font-size: 11px;
    font-weight: 600;
}

.hs-toast-action:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-toast-action.err {
    background: #FF5D5D;
    color: #FFFFFF;
}

.hs-toast-action.err:hover {
    filter: brightness(1.08);
}


/* ═══════════════ Countdown overlay (ADR-0015) ═══════════════ */

window.hs-countdown-window,
window.hs-countdown-window > * {
    background: transparent;
}

.hs-cd-circle {
    border-radius: 999px;
    border: 2px solid rgba(255, 255, 255, 0.18);
    background: radial-gradient(circle, rgba(94, 230, 208, 0.14), rgba(6, 8, 12, 0.42) 75%);
}

window.hs-countdown-window.hs-mode-rec .hs-cd-circle {
    background: radial-gradient(circle, rgba(255, 93, 93, 0.16), rgba(6, 8, 12, 0.42) 75%);
}

.hs-cd-num {
    color: #FFFFFF;
    font-size: 130px;
    font-weight: 200;
    text-shadow: 0 8px 40px rgba(0, 0, 0, 0.5);
}

.hs-cd-cancel {
    border: 1px solid rgba(255, 255, 255, 0.10);
    background: rgba(0, 0, 0, 0.5);
    color: #EDEEF2;
    border-radius: 999px;
    padding: 9px 16px;
    min-height: 0;
    font-family: "JetBrains Mono", "Fira Mono", monospace;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
}

.hs-cd-cancel:hover {
    background: rgba(0, 0, 0, 0.7);
}


/* ═══════════════ Full recording HUD (ADR-0016) ═══════════════ */

window.hs-hud-window,
window.hs-hud-window > * {
    background: transparent;
}

window.hs-hud-window .hs-hud {
    background: rgba(19, 20, 26, 0.90);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-radius: 999px;
    padding: 7px 8px 7px 14px;
}

.hs-hb {
    border: none;
    background: transparent;
    border-radius: 10px;
    padding: 0;
    min-height: 0;
    transition: all 120ms;
}

.hs-hb:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-hb:disabled {
    opacity: 0.3;
}

.hs-hb.off {
    background: rgba(255, 93, 93, 0.16);
}

.hs-hud-dot.paused {
    animation: none;
    opacity: 0.4;
}


/* ═══════════════ Annotation editor (ADR-0018) ═══════════════ */

window.hs-annot-window {
    background: rgba(19, 20, 26, 0.86);
}

.hs-annot-top {
    padding: 10px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.10);
}

.hs-annot-title {
    color: #EDEEF2;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.01em;
}

.hs-annot-close {
    border: none;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 7px;
    padding: 0;
    min-height: 0;
}

.hs-annot-close:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-annot-tools {
    padding: 10px 8px;
    border-right: 1px solid rgba(255, 255, 255, 0.10);
}

.hs-atool {
    border: none;
    background: transparent;
    border-radius: 9px;
    padding: 0;
    min-height: 0;
    transition: all 120ms;
}

.hs-atool:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-atool:checked {
    background: rgba(94, 230, 208, 0.16);
    box-shadow: 0 0 0 1px #5EE6D0 inset;
}

.hs-annot-foot {
    padding: 10px 14px;
    border-top: 1px solid rgba(255, 255, 255, 0.10);
}

.hs-af-sw {
    border: 2px solid transparent;
    border-radius: 999px;
    padding: 0;
    min-height: 0;
}

.hs-af-sw:checked {
    border-color: #FFFFFF;
}

.hs-af-sw.sw0 { background: #5EE6D0; }
.hs-af-sw.sw1 { background: #FF5D5D; }
.hs-af-sw.sw2 { background: #FFD23F; }
.hs-af-sw.sw3 { background: #7CA8FF; }
.hs-af-sw.sw4 { background: #FFFFFF; }

.hs-af-btn {
    border: none;
    border-radius: 9px;
    padding: 0 14px;
    min-height: 34px;
    font-size: 12px;
    font-weight: 600;
    transition: filter 120ms;
}

.hs-af-btn.ghost {
    background: rgba(255, 255, 255, 0.05);
    color: #EDEEF2;
    margin-right: 8px;
}

.hs-af-btn.ghost:hover {
    background: rgba(255, 255, 255, 0.09);
}

.hs-af-btn.solid {
    background: #5EE6D0;
    color: #06231F;
}

.hs-af-btn.solid:hover {
    filter: brightness(1.07);
}

entry.hs-annot-entry {
    background: rgba(19, 20, 26, 0.92);
    color: #EDEEF2;
    border: 1px solid #5EE6D0;
    border-radius: 7px;
    padding: 5px 8px;
    font-size: 13px;
    min-height: 0;
    caret-color: #5EE6D0;
}
"#;
