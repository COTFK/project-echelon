//! Video preview component for displaying completed replay videos.

use dioxus::prelude::*;

/// Displays a video preview with download option.
#[component]
pub fn VideoPreview(video_url: String) -> Element {
    rsx! {
        div {
            class: "card bg-base-200 border-base-300 border shadow-md mt-4 md:max-w-1/2",
            div {
                class: "card-body px-4",
                h2 { class: "card-title", "Preview" }
                video {
                    class: "w-full rounded-lg",
                    controls: true,
                    autoplay: true,
                    src: "{video_url}",
                }
                div {
                    class: "card-actions justify-end items-center mt-2",
                    span {
                        class: "text-base-content/70",
                        "Download expires in 1 hour"
                    }
                    a {
                        class: "btn btn-primary px-3",
                        href: "{video_url}",
                        download: true,
                        i { class: "bi bi-download me-1" }
                        "Download"
                    }
                }
            }
        }
    }
}
