//! Upload form component for replay file submission.

use std::time::Duration;

use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_sdk_time::use_interval;

use crate::api::{ApiClient, validate_replay_file};
use crate::components::Hero;
use crate::components::status_display::{LoadingSpinner, StatusDisplay};
use crate::components::video_preview::VideoPreview;
use crate::types::{REPLAY_EXTENSION, ReplayError, ReplayStatus};

/// Main upload form component.
#[component]
pub fn UploadForm() -> Element {
    let mut replay_id = use_signal(String::new);
    let mut status = use_signal(ReplayStatus::default);
    let api_client = use_hook(ApiClient::default);
    let mut show_hero = use_signal(|| true);
    let mut video_url = use_signal(String::new);

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

                match api_client.upload_replay(data.to_vec()).await {
                    Ok(id) => {
                        tracing::info!("Upload successful, replay ID: {id}");
                        replay_id.set(id.clone());
                        match api_client.get_status(&id).await {
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
                            class: "flex justify-center text-base",
                            StatusDisplay { status: status() }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| status.set(ReplayStatus::Idle),
                            "Try again"
                        }
                    } else if matches!(status(), ReplayStatus::Completed(_)) {
                        legend { class: "fieldset-legend text-base", "Done!" }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| status.set(ReplayStatus::Idle),
                            "Convert another video"
                        }
                    } else {
                        legend { class: "fieldset-legend text-base", "Converting..." }
                        LoadingSpinner {}
                        div {
                            class: "flex justify-center text-base mt-2",
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
