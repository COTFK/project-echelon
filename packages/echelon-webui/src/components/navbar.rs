//! Navigation bar component.

use dioxus::prelude::*;

/// Application navigation bar.
#[component]
pub fn NavBar() -> Element {
    rsx! {
        nav {
            class: "navbar sticky bg-base-300 shadow-sm",
            a {
                href: "/",
                class: "btn btn-ghost text-xl",
                img {
                    class: "size-8",
                    src: asset!("/assets/favicon.png")
                }
                "Project Echelon"
            }
        }
    }
}
