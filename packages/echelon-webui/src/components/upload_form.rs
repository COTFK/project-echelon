use dioxus::prelude::*;

use crate::types::REPLAY_EXTENSION;
use crate::types::VideoPreset;

#[component]
pub fn UploadForm(
    on_submit: EventHandler<FormEvent>,
    top_down_view: Signal<bool>,
    swap_players: Signal<bool>,
    game_speed: Signal<f64>,
    video_preset: Signal<VideoPreset>,
    show_advanced_settings: Signal<bool>,
) -> Element {
    rsx!(
        form {
            onsubmit: on_submit,
            fieldset {
                class: "fieldset bg-base-200 border-base-300 rounded-box min-w-64 md:w-sm lg:w-md border pb-6 pt-4 px-6 flex flex-col gap-4",
                legend { class: "fieldset-legend text-base", "Upload your replay" }
                div {
                    class: "flex flex-col justify-center items-center",
                    input {
                        class: "file-input",
                        name: "replay",
                        r#type: "file",
                        accept: REPLAY_EXTENSION,
                        required: true,
                    }
                    label { class: "label pt-2", "*{REPLAY_EXTENSION} file, max size 10MB" }
                }

                // Advanced settings dropdown
                div {
                    class: "collapse collapse-arrow bg-base-100 bg-opacity-50 ",
                    input {
                        r#type: "checkbox",
                        onchange: move |evt| show_advanced_settings.set(evt.checked()),
                    }
                    div { class: "collapse-title text-sm font-medium",
                        "Advanced Settings (Optional)"
                    }
                    div {
                        class: "collapse-content flex flex-col gap-3",
                        // Top-down view checkbox
                        label {
                            class: "label cursor-pointer gap-2",
                            input {
                                class: "checkbox checkbox-sm",
                                r#type: "checkbox",
                                onchange: move |evt| top_down_view.set(evt.checked()),
                            }
                            span { class: "label-text text-sm", "Top-down view" }
                        }

                        // Swap players checkbox
                        label {
                            class: "label cursor-pointer gap-2",
                            input {
                                class: "checkbox checkbox-sm",
                                r#type: "checkbox",
                                onchange: move |evt| swap_players.set(evt.checked()),
                            }
                            span { class: "label-text text-sm", "Swap players" }
                        }

                        // Game speed dropdown
                        label {
                            class: "label",
                            span { class: "label-text text-sm", "Game Speed" }
                        }
                        select {
                            class: "select select-bordered select-sm w-full",
                            onchange: move |evt| {
                                if let Ok(val) = evt.value().parse::<f64>() {
                                    game_speed.set(val);
                                }
                            },
                            option { value: "0.5", "Slowest (0.5x)" }
                            option { value: "0.75", "Slow (0.75x)" }
                            option { value: "1.0", selected: true, "Normal (1x)" }
                            option { value: "1.5", "Fast (1.5x)" }
                            option { value: "2.0", "Faster (2x)" }
                            option { value: "3.0", "Very Fast (3x)" }
                            option { value: "10.0", "Timelapse (10x)" }
                        }
                        label { class: "label", span { class: "label-text text-sm", "Video preset" } }
                        select {
                            class: "select select-bordered select-sm w-full",
                            onchange: move |evt| {
                                video_preset.set(VideoPreset::from_str(&evt.value()));
                            },
                            option { value: VideoPreset::FileSize.as_str(), "File-size optimized" }
                            option { value: VideoPreset::Balanced.as_str(), selected: true, "Balanced (default)" }
                            option { value: VideoPreset::Quality.as_str(), "High quality" }
                        }
                    }
                }

                button { class: "btn btn-primary", r#type: "submit", "Convert" }
                p { class: "text-xs text-center opacity-60 mt-2",
                    "By uploading, you agree to our "
                    a {
                        href: "/terms",
                        class: "link",
                        "Terms of Service"
                    }
                    " and "
                    a {
                        href: "/privacy",
                        class: "link",
                        "Privacy Policy"
                    }
                    "."
                }
            }
        }
    )
}
