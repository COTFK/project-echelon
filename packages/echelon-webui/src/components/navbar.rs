//! Navigation bar component.

use dioxus::prelude::*;

/// Application navigation bar.
#[component]
pub fn NavBar() -> Element {
    rsx! {
        nav {
            class: "navbar absolute bg-base-300 shadow-sm",
            a {
                href: "/",
                class: "btn btn-ghost text-xl",
                "Project Echelon"
            }
        }
    }
}
