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
                        href: "{video_url}?download=1",
                        download: true,
                        svg {
                            xmlns: "http://www.w3.org/2000/svg",
                            width: "1em",
                            height: "1em",
                            fill: "currentColor",
                            class: "me-1",
                            view_box: "0 0 16 16",
                            path {
                                d: "M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5",
                            }
                            path {
                                d: "M7.646 11.854a.5.5 0 0 0 .708 0l3-3a.5.5 0 0 0-.708-.708L8.5 10.293V1.5a.5.5 0 0 0-1 0v8.793L5.354 8.146a.5.5 0 1 0-.708.708z"
                            }
                        }
                        "Download"
                    }
                }
            }
        }
    }
}
