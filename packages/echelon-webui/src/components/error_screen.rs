use dioxus::prelude::*;

use crate::types::ReplayError;

#[component]
pub fn ErrorScreen(error: ReplayError, reset_form: EventHandler<MouseEvent>) -> Element {
    let class = if matches!(error, ReplayError::QueueFull) {
        "text-warning"
    } else {
        "text-error"
    };

    rsx!(
        fieldset {
            class: "fieldset bg-base-200 border-base-300 rounded-box min-w-64 md:w-sm lg:w-md border pb-6 pt-4 px-6 flex flex-col gap-4",
            legend { class: "fieldset-legend text-base", "Error!" }
            div {
                class: "flex items-center text-base flex-col gap-2",
                 p { class, "{error}" }
            }
            button {
                class: "btn btn-primary",
                onclick: reset_form,
                "Try again"
            }
        }
    )
}