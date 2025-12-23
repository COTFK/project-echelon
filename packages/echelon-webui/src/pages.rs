use crate::components::{Footer, NavBar, PrivacyPolicy, TermsOfService, UploadForm};
use dioxus::prelude::*;

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

#[component]
pub fn TermsOfServicePage() -> Element {
    rsx!(
        NavBar {}
        TermsOfService {}
        Footer {}
    )
}
