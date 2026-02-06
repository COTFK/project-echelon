//! Hero component - shown when page is loaded; hidden when processing a replay

use dioxus::prelude::*;

const IGNIS_LOGO: Asset = asset!("/assets/ignis_logo.png");

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            class: "hero lg:max-w-1/3",
            div {
                class: "hero-content text-center text-base-content flex-col gap-8",
                div {
                    class: "flex flex-row items-center justify-evenly",
                    img {
                        class: "h-[60px] lg:h-[97px]",
                        src: IGNIS_LOGO
                    }
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "1em",
                        height: "1em",
                        fill: "currentColor",
                        class: "text-6xl lg:text-8xl",
                        view_box: "0 0 16 16",
                        path {
                            fill_rule: "evenodd",
                            d: "M4 8a.5.5 0 0 1 .5-.5h5.793L8.146 5.354a.5.5 0 1 1 .708-.708l3 3a.5.5 0 0 1 0 .708l-3 3a.5.5 0 0 1-.708-.708L10.293 8.5H4.5A.5.5 0 0 1 4 8"
                        }
                    }
                    svg {
                        xmlns: "http://www.w3.org/2000/svg",
                        width: "1em",
                        height: "1em",
                        fill: "currentColor",
                        class: "text-6xl lg:text-8xl",
                        view_box: "0 0 16 16",
                        path {
                            fill_rule: "evenodd",
                            d: "M0 1a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H1a1 1 0 0 1-1-1zm4 0v6h8V1zm8 8H4v6h8zM1 1v2h2V1zm2 3H1v2h2zM1 7v2h2V7zm2 3H1v2h2zm-2 3v2h2v-2zM15 1h-2v2h2zm-2 3v2h2V4zm2 3h-2v2h2zm-2 3v2h2v-2zm2 3h-2v2h2z"
                        }
                    }
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
