use crate::components::{Footer, NavBar, PrivacyPolicy, TermsOfService, ConverterFlow};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx!(
        NavBar {}
        ConverterFlow {}
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
