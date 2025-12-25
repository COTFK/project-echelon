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
use pages::TermsOfServicePage;

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/")]
    UploadPage,

    #[route("/privacy")]
    PrivacyPolicyPage,

    #[route("/terms")]
    TermsOfServicePage,
}

fn main() {
    dioxus::launch(App);
}

/// Root application component.
#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: asset!("/assets/favicon.png") }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }
        Router::<Route> {}
    }
}
