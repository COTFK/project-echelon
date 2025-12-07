//! Status display component for showing replay processing status.

use dioxus::prelude::*;

use crate::types::{ReplayError, ReplayStatus};

/// Displays the current status of replay processing.
#[component]
pub fn StatusDisplay(status: ReplayStatus) -> Element {
    match status {
        ReplayStatus::Idle => rsx! {},
        ReplayStatus::Uploading => rsx! { p { class: "", "Uploading..." } },
        ReplayStatus::Queued(position) => {
            rsx! { p { class: "", "Queued at position {position}..." } }
        }
        ReplayStatus::Processing => rsx! { p { class: "", "Processing..." } },
        ReplayStatus::Completed(_) => rsx! { p { class: "text-success", "Done!" } },
        ReplayStatus::Error(ref error) => {
            let class = if matches!(error, ReplayError::QueueFull) {
                "text-warning"
            } else {
                "text-error"
            };
            rsx! { p { class, "{error}" } }
        }
    }
}

/// Loading spinner shown during active processing.
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! { span { class: "loading loading-spinner mx-auto" } }
}
