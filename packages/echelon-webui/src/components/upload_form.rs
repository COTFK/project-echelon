//! Upload form component for replay file submission.

use std::time::Duration;

use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_sdk_time::use_interval;

use crate::api::{ApiClient, validate_replay_file};
use crate::components::Hero;
use crate::components::status_display::{LoadingSpinner, StatusDisplay};
use crate::components::video_preview::VideoPreview;
use crate::types::{REPLAY_EXTENSION, ReplayConfig, ReplayError, ReplayStatus, VideoPreset};

/// Main upload form component.
#[component]
pub fn UploadForm() -> Element {
    let mut replay_id = use_signal(String::new);
    let mut status = use_signal(ReplayStatus::default);
    let api_client = use_hook(ApiClient::default);
    let mut show_hero = use_signal(|| true);
    let mut video_url = use_signal(String::new);

    // Advanced config options
    let mut top_down_view = use_signal(|| false);
    let mut swap_players = use_signal(|| false);
    let mut game_speed = use_signal(|| 1.0);
    let mut video_preset = use_signal(|| VideoPreset::Balanced);
    let mut show_advanced = use_signal(|| false);

    // Poll status while processing
    use_interval(Duration::from_secs(1), {
        let api_client = api_client.clone();
        move |_| {
            let id = replay_id.read().clone();
            let current_status = status.read().clone();

            if !id.is_empty() && current_status.should_poll() {
                let api_client = api_client.clone();
                spawn(async move {
                    match api_client.get_status(&id).await {
                        Ok(new_status) => {
                            if let ReplayStatus::Completed(ref id) = new_status {
                                show_hero.set(false);
                                video_url.set(api_client.download_url(id));
                            }

                            status.set(new_status);
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch status: {e}");
                            status.set(ReplayStatus::Error(e));
                        }
                    }
                });
            }
        }
    });

    let handle_submit = {
        let api_client = api_client.clone();
        move |evt: FormEvent| {
            let api_client = api_client.clone();
            async move {
                evt.prevent_default();

                let Some(FormValue::File(Some(file))) = evt.get_first("replay") else {
                    return;
                };

                let data = match file.read_bytes().await {
                    Ok(data) => data,
                    Err(e) => {
                        tracing::error!("Failed to read file: {e:?}");
                        status.set(ReplayStatus::Error(ReplayError::Validation(
                            "Failed to read file".to_owned(),
                        )));
                        return;
                    }
                };

                if let Err(e) = validate_replay_file(&file.name(), &data) {
                    status.set(ReplayStatus::Error(e));
                    return;
                }

                status.set(ReplayStatus::Uploading);

                // Step 1: Create the replay job with config
                let config = ReplayConfig {
                    top_down_view: top_down_view(),
                    swap_players: swap_players(),
                    game_speed: game_speed(),
                    video_preset: video_preset(),
                };

                let task_id = match api_client.create_replay(&config).await {
                    Ok(id) => {
                        tracing::info!("Replay created with ID: {id}");
                        id
                    }
                    Err(e) => {
                        tracing::error!("Failed to create replay: {e}");
                        status.set(ReplayStatus::Error(e));
                        return;
                    }
                };

                // Step 2: Upload the replay file
                match api_client.upload_replay(&task_id, data.to_vec()).await {
                    Ok(_) => {
                        tracing::info!("Upload successful, replay ID: {task_id}");
                        replay_id.set(task_id.clone());
                        match api_client.get_status(&task_id).await {
                            Ok(initial_status) => status.set(initial_status),
                            Err(e) => status.set(ReplayStatus::Error(e)),
                        }
                    }
                    Err(e) => {
                        tracing::error!("Upload failed: {e}");
                        status.set(ReplayStatus::Error(e));
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "flex flex-col md:flex-row gap-4 items-center md:justify-evenly w-full flex-1 px-4 py-8",
            if show_hero() {
                Hero {  }
            }
            form {
                onsubmit: handle_submit,
                fieldset {
                    class: "fieldset bg-base-200 border-base-300 rounded-box min-w-64 md:w-sm lg:w-md border pb-6 pt-4 px-6 flex flex-col gap-4",
                    if matches!(status(), ReplayStatus::Idle) {
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
                                onchange: move |evt| show_advanced.set(evt.checked()),
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
                    } else if matches!(status(), ReplayStatus::Error(_)) {
                        legend { class: "fieldset-legend text-base", "Error" }
                        div {
                            class: "flex items-center text-base flex-col gap-2",
                            StatusDisplay { status: status() }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                status.set(ReplayStatus::Idle);
                                top_down_view.set(false);
                                swap_players.set(false);
                                game_speed.set(1.0);
                                video_preset.set(VideoPreset::Balanced);
                                show_advanced.set(false);
                            },
                            "Try again"
                        }
                    } else if matches!(status(), ReplayStatus::Completed(_)) {
                        legend { class: "fieldset-legend text-base", "Done!" }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                status.set(ReplayStatus::Idle);
                                top_down_view.set(false);
                                swap_players.set(false);
                                game_speed.set(1.0);
                                video_preset.set(VideoPreset::Balanced);
                                show_advanced.set(false);
                            },
                            "Convert another video"
                        }
                    } else {
                        legend { class: "fieldset-legend text-base", "Converting..." }
                        LoadingSpinner {}
                        div {
                            class: "flex items-center text-base mt-2 flex-col gap-2",
                            StatusDisplay { status: status() }
                        }
                    }
                }
            }
            if matches!(status(), ReplayStatus::Completed(_)) {
                VideoPreview { video_url }
            }
        }
    }
}
