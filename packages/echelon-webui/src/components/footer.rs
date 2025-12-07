//! Footer component

use chrono::Datelike;
use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    let year = chrono::Local::now().year();

    rsx!(
        footer {
            class: "absolute bottom-0 footer sm:footer-horizontal bg-base-300 text-neutral-content items-center p-4 justify-between",
            aside {
                class: "items-center",
                p {
                    "© {year} Circle of the Fire Kings"
                }
            }
            nav {
                div {
                    class: "grid grid-flow-col gap-4 md:place-self-center md:justify-self-end",
                    a {
                        href: "https://discord.gg/8JtxHUAdGq",
                        i { class: "bi bi-discord text-2xl"}
                    }
                    a {
                        href: "https://git.arqalite.org/COTFK/project-echelon",
                        i { class: "bi bi-git text-2xl"}
                    }
                }
            }
        }
    )
}
