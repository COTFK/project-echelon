use dioxus::prelude::*;

#[component]
pub fn TermsOfService() -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-4xl prose prose-invert",
            // Header
            h1 { class: "text-4xl font-bold mb-2", "Terms of Service" }
            p { class: "text-sm opacity-70 mb-8",
                strong { "Last Updated: " }
                "2025-12-23"
            }

            // Section 1
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "1. Acceptance of Terms" }
            p {
                "By using Project Echelon (\"the Service\"), you agree to these Terms of Service. 
                If you do not agree, please do not use the Service."
            }

            // Section 2
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "2. Description of Service" }
            p {
                "Project Echelon is a free service that converts Project Ignis: EDOPro replay files ("
                code { class: "badge badge-neutral", ".yrpX" }
                ") into video format. The Service is provided \"as is\" without warranty."
            }

            // Section 3
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "3. Acceptable Use" }
            p { "You agree to:" }
            ul { class: "list-disc list-inside ml-4 space-y-1",
                li { "Only upload legitimate EDOPro replay files" }
                li { "Not attempt to abuse, overload, or exploit the Service" }
                li { "Not use the Service for any illegal purpose" }
                li { "Respect the rate limits in place (5 uploads per 60 seconds)" }
            }

            // Section 4
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "4. Intellectual Property" }
            p {
                "You retain ownership of your replay files. The generated videos are yours to use. 
                Project Echelon does not claim ownership over user-submitted content."
            }
            p { class: "mt-2",
                "Yu-Gi-Oh! and related content are trademarks of Konami. EDOPro is developed by Project Ignis. 
                This Service is not affiliated with or endorsed by Konami or Project Ignis."
            }

            // Section 5
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "5. Disclaimer of Warranties" }
            div { class: "alert alert-soft alert-warning",
                p {
                    "THE SERVICE IS PROVIDED \"AS IS\" WITHOUT WARRANTIES OF ANY KIND, EXPRESS OR IMPLIED, 
                    INCLUDING BUT NOT LIMITED TO WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, 
                    OR NON-INFRINGEMENT. WE DO NOT GUARANTEE UNINTERRUPTED OR ERROR-FREE OPERATION."
                }
            }

            // Section 6
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "6. Limitation of Liability" }
            p {
                "To the maximum extent permitted by law, Project Echelon and its operators shall not be liable 
                for any indirect, incidental, special, consequential, or punitive damages arising from your use 
                of the Service."
            }

            // Section 7
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "7. Service Availability" }
            p {
                "We reserve the right to modify, suspend, or discontinue the Service at any time without notice. 
                We are not liable for any modification, suspension, or discontinuation."
            }

            // Section 8
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "8. Changes to Terms" }
            p {
                "We may update these Terms at any time. Continued use of the Service after changes 
                constitutes acceptance of the updated Terms."
            }

            // Section 9
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "9. Contact" }
            p { "For questions about these Terms:" }
            ul { class: "list-disc list-inside ml-4 space-y-2",
                li {
                    strong { "Email: " }
                    a {
                        href: "mailto:feedback@arqalite.org",
                        class: "link link-info",
                        "feedback@arqalite.org"
                    }
                }
                li {
                    strong { "Discord: " }
                    a {
                        href: "https://discord.gg/8JtxHUAdGq",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        class: "link link-info",
                        "Join our Discord server"
                    }
                }
            }
        }
    }
}
