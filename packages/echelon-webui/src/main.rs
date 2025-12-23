//! Project Echelon Web UI
//!
//! A web interface for uploading Yu-Gi-Oh! replay files and converting them to video.

mod api;
mod components;
mod types;
mod pages;

use dioxus::prelude::*;

use pages::UploadPage;
use pages::PrivacyPolicyPage;

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    UploadPage,

    #[route("/privacy-policy")]
    PrivacyPolicyPage
}

fn main() {
    dioxus::launch(App);
}

/// Root application component.
#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: "https://cdn.jsdelivr.net/npm/daisyui@5" }
        document::Stylesheet { href: "https://cdn.jsdelivr.net/npm/daisyui@5/themes.css" }
        document::Stylesheet { href: "https://cdn.jsdelivr.net/npm/bootstrap-icons@1.13.1/font/bootstrap-icons.min.css" }
        document::Script { src: "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4" }
        div {
            class: "relative flex flex-col h-dvh w-dvw overflow-x-hidden overflow-y-auto",
            "data-theme": "business",
            Router::<Route> {}
        }
    }
}
