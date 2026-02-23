//! Status display component for showing replay processing status.

use dioxus::prelude::*;

use crate::types::ReplayError; 
use crate::types::ProcessingStatus;

/// Displays the current status of replay processing.
#[component]
pub fn ProcessingScreen(status: ProcessingStatus) -> Element {
    rsx!(
        fieldset {
            class: "fieldset bg-base-200 border-base-300 rounded-box min-w-64 md:w-sm lg:w-md border pb-6 pt-4 px-6 flex flex-col gap-4",
            legend { class: "fieldset-legend text-base", "Converting..." }
            span { class: "loading loading-spinner mx-auto" }
            div {
                class: "flex items-center text-base mt-2 flex-col gap-2",
                match status {
                    ProcessingStatus::Idle => rsx! {},
                    ProcessingStatus::Uploading => rsx! { p { class: "", "Uploading..." } },
                    ProcessingStatus::Queued {
                        position,
                        estimate_minutes,
                    } => {
                        rsx! {
                            p { class: "", "Queued at position {position}." }
                            p { class: "", "(ETA: {estimate_minutes} min.)" }
                        }
                    }
                    ProcessingStatus::Processing { estimate_minutes } => {
                        rsx! {
                            p { class: "", "Processing..." }
                            p { class: "", "(ETA: {estimate_minutes} min.)" }
                        }
                    }
                    ProcessingStatus::Completed(_) => rsx! { p { class: "text-success", "Done!" } },
                    ProcessingStatus::Error(ref error) => {
                        let class = if matches!(error, ReplayError::QueueFull) {
                            "text-warning"
                        } else {
                            "text-error"
                        };
                        rsx! { p { class, "{error}" } }
                    }
                }
            }
        }
    )
}
