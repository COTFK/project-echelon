use crate::components::{
    ConverterFlow, DiscordBotHelp, Footer, NavBar, PrivacyPolicy, TermsOfService, WebsiteHelp,
};
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

#[component]
pub fn HelpPage() -> Element {
    rsx!(
        NavBar {}
        div { 
            class: "container mx-auto px-4 py-8 max-w-4xl prose prose-invert",
            h1 { class: "text-4xl font-bold", "Help" }
            div {
                class: "tabs tabs-box tabs-lg mt-8",
                input {
                    r#type: "radio", 
                    name: "help", 
                    class: "tab",
                    aria_label: "General",
                    checked: true,
                }
                div {
                    class: "tab-content bg-base-100 border-base-300 p-6",
                    WebsiteHelp {}
                }
                input {
                    r#type: "radio", 
                    name: "help", 
                    class: "tab",
                    aria_label: "Discord Bot"
                }
                div {
                    class: "tab-content bg-base-100 border-base-300 p-6",
                    DiscordBotHelp {}
                }
            }
        }
        Footer {}
    )
}
