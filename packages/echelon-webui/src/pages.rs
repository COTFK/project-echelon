use dioxus::prelude::*;
use crate::components::{Footer, NavBar, UploadForm, PrivacyPolicy};

#[component]
pub fn UploadPage() -> Element {
    rsx!(
        NavBar {}
        UploadForm {}
        Footer {}
    )
}

#[component]
pub fn PrivacyPolicyPage() -> Element {
    rsx!(
        NavBar {}
        PrivacyPolicy {}
        Footer {}
    )
}