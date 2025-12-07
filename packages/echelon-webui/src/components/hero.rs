//! Hero component - shown when page is loaded; hidden when processing a replay

use dioxus::prelude::*;

const IGNIS_LOGO: Asset = asset!("/assets/ignis_logo.png");

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            class: "hero max-w-1/3",
            div {
                class: "hero-content text-center text-base-content flex-col gap-8",
                div {
                    class: "flex flex-row items-center justify-evenly",
                    img {
                        class: "h-[97px]",
                        src: IGNIS_LOGO
                    }
                    i { class: "bi bi-arrow-right-short text-8xl"}
                    i { class: "bi bi-film text-8xl" }
                }
                h1 {
                    class: "text-3xl font-bold",
                    "Convert EDOPro replays into videos"
                }
                p {
                    class: "text-base text-base-content/70",
                    "Two clicks is all it takes - select your replay file and start converting!"
                }
            }
        }
    }
}
