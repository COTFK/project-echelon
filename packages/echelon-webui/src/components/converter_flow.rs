use std::time::Duration;

use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_sdk_time::use_interval;

use crate::api::ApiClient;
use crate::api::validate_replay_file;
use crate::components::Hero;
use crate::components::UploadForm;
use crate::components::ProcessingScreen;
use crate::components::CompletedScreen;
use crate::components::ErrorScreen;
use crate::types::ProcessingStatus;
use crate::types::ReplayConfig;
use crate::types::ReplayError;
use crate::types::VideoPreset;

#[component]
pub fn ConverterFlow() -> Element {
    let mut replay_id = use_signal(String::new);
    let mut status = use_signal(ProcessingStatus::default);
    let api_client = use_hook(ApiClient::default);
    let mut show_hero = use_signal(|| true);
    let mut video_url = use_signal(String::new);

    // Advanced settings
    let mut show_advanced_settings = use_signal(|| false);
    let mut top_down_view = use_signal(|| false);
    let mut swap_players = use_signal(|| false);
    let mut game_speed = use_signal(|| 1.0);
    let mut video_preset = use_signal(|| VideoPreset::Balanced);

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
                            if let ProcessingStatus::Completed(ref id) = new_status {
                                show_hero.set(false);
                                video_url.set(api_client.download_url(id));
                            }

                            status.set(new_status);
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch status: {e}");
                            status.set(ProcessingStatus::Error(e));
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
                        status.set(ProcessingStatus::Error(ReplayError::Validation(
                            "Failed to read file".to_owned(),
                        )));
                        return;
                    }
                };

                if let Err(e) = validate_replay_file(&file.name(), &data) {
                    status.set(ProcessingStatus::Error(e));
                    return;
                }

                status.set(ProcessingStatus::Uploading);

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
                        status.set(ProcessingStatus::Error(e));
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
                            Err(e) => status.set(ProcessingStatus::Error(e)),
                        }
                    }
                    Err(e) => {
                        tracing::error!("Upload failed: {e}");
                        status.set(ProcessingStatus::Error(e));
                    }
                }
            }
        }
    };

    let reset_form = move |_| {
        status.set(ProcessingStatus::Idle);
        top_down_view.set(false);
        swap_players.set(false);
        game_speed.set(1.0);
        video_preset.set(VideoPreset::Balanced);
        show_advanced_settings.set(false);
    };

    rsx!(
        div {
            class: "flex flex-col lg:flex-row gap-4 items-center justify-evenly w-full flex-1 px-4 py-4",
            match status() {
                ProcessingStatus::Idle => rsx!(
                    if show_hero() {
                        Hero {  }
                    }
                    UploadForm {
                        on_submit: handle_submit,
                        top_down_view,
                        swap_players,
                        game_speed,
                        video_preset,
                        show_advanced_settings,
                    }
                ),
                ProcessingStatus::Completed(_) => rsx!(
                    CompletedScreen {
                        video_url,
                        reset_form
                    }
                ),
                ProcessingStatus::Error(ref error) => rsx!(
                    ErrorScreen {error: error.clone(), reset_form}
                ),
                _ => rsx!(
                    ProcessingScreen { status: status() }
                )
            }
        }
    )
}





