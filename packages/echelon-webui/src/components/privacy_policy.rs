use dioxus::prelude::*;

#[component]
pub fn PrivacyPolicy() -> Element {
    rsx! {
        div { class: "container mx-auto px-4 py-8 max-w-4xl prose prose-invert",
            // Header
            h1 { class: "text-4xl font-bold mb-2", "Privacy Policy" }
            p { class: "text-sm opacity-70 mb-8",
                strong { "Last Updated: " }
                "2025-12-23"
            }

            p { class: "lead",
                "Project Echelon (\"we\", \"our\", \"the Service\") is a replay-to-video conversion service
                for Project Ignis: EDOPro. This policy explains what data we collect and how we use it."
            }

            // Section 1
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "1. Information We Collect" }

            h3 { class: "text-xl font-medium mt-6 mb-3", "1.1 Replay Files" }
            p {
                "When you submit a "
                code { class: "badge badge-neutral", ".yrpX" }
                " replay file through our Discord bot or web interface, we process the file to generate an MP4 video. Replay files may contain:"
            }
            ul { class: "list-disc list-inside ml-4 space-y-1",
                li { "In-game usernames and player information from the replay" }
                li { "Card plays and game actions" }
            }

            h3 { class: "text-xl font-medium mt-6 mb-3", "1.2 Technical Data (Automatic)" }
            ul { class: "list-disc list-inside ml-4 space-y-2",
                li {
                    strong { "IP Addresses: " }
                    "Used solely for rate limiting (5 uploads per 60 seconds) to prevent abuse. IP addresses are not logged or stored persistently."
                }
                li {
                    strong { "Discord User IDs: " }
                    "When using our Discord bot, we receive your Discord User ID to send you the completed video. We do not store this beyond the processing session."
                }
            }

            // Section 2
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "2. How We Use Your Data" }
            p { "We use the collected data exclusively for:" }
            ul { class: "list-disc list-inside ml-4 space-y-1",
                li { "Converting your replay files to MP4 video" }
                li { "Delivering the completed video back to you" }
                li { "Preventing service abuse through rate limiting" }
            }

            div { class: "alert alert-soft alert-warning mt-4",
                div {
                    strong { "We do NOT:" }
                    ul { class: "list-disc list-inside ml-4 mt-2 space-y-1",
                        li { "Sell or share your data with third parties" }
                        li { "Store replay files or videos permanently (auto-deleted after 1 hour)" }
                        li { "Create user accounts or profiles" }
                        li { "Track usage across sessions" }
                        li { "Use cookies or similar tracking technologies" }
                    }
                }
            }

            // Section 3
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "3. Data Retention" }
            div { class: "overflow-x-auto",
                table { class: "table table-zebra w-full",
                    thead {
                        tr {
                            th { "Data Type" }
                            th { "Retention Period" }
                        }
                    }
                    tbody {
                        tr {
                            td { "Replay files" }
                            td { "Deleted automatically after 1 hour" }
                        }
                        tr {
                            td { "Generated videos" }
                            td { "Deleted automatically after 1 hour" }
                        }
                        tr {
                            td { "IP addresses" }
                            td { "Not stored (used transiently for rate limiting only)" }
                        }
                        tr {
                            td { "Discord User IDs" }
                            td { "Not stored beyond the active processing session" }
                        }
                    }
                }
            }

            // Section 4
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "4. Data Processing Location" }
            p {
                "Your data is processed on our servers located in Germany (Hetzner).
                All processing is done in memory and temporary files are automatically cleaned up."
            }

            // Section 5
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "5. Third-Party Services" }

            h3 { class: "text-xl font-medium mt-6 mb-3", "Discord" }
            p {
                "If you use our Discord bot, "
                a {
                    href: "https://discord.com/privacy",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    class: "link link-info",
                    "Discord's Privacy Policy"
                }
                " applies to your use of Discord. We only receive the minimum data necessary from Discord to provide the service."
            }

            h3 { class: "text-xl font-medium mt-6 mb-3", "Project Ignis: EDOPro" }
            p {
                "This service uses a modified version of EDOPro to process replays.
                The replay content is determined by EDOPro's replay format."
            }

            // Section 6
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "6. Data Security" }
            ul { class: "list-disc list-inside ml-4 space-y-1",
                li { "All data processing occurs in isolated, temporary environments" }
                li { "Replay and video files are stored in memory and temporary directories" }
                li { "Automatic cleanup removes all job data within 1 hour" }
                li { "Rate limiting protects against abuse" }
            }

            // Section 7
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "7. Your Rights" }
            p {
                "Since we do not store personal data persistently, there is typically no
                data to request, modify, or delete. If you have concerns, contact us using the information below."
            }

            // Section 8
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "8. Children's Privacy" }
            p {
                "This service does not knowingly collect data from children under 13.
                The service is intended for users of the EDOPro game."
            }

            // Section 9
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "9. Changes to This Policy" }
            p {
                "We may update this policy occasionally. Continued use of the Service
                after changes constitutes acceptance of the updated policy."
            }

            // Section 10
            h2 { class: "text-2xl font-semibold mt-8 mb-4", "10. Contact" }
            p { "For privacy questions or concerns:" }
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

            // Open Source Notice
            div { class: "divider mt-8" }
            div { class: "alert mt-4",
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "1em",
                    height: "1em",
                    fill: "currentColor",
                    class: "text-2xl",
                    view_box: "0 0 16 16",
                    path {
                        d: "M15.698 7.287 8.712.302a1.03 1.03 0 0 0-1.457 0l-1.45 1.45 1.84 1.84a1.223 1.223 0 0 1 1.55 1.56l1.773 1.774a1.224 1.224 0 0 1 1.267 2.025 1.226 1.226 0 0 1-2.002-1.334L8.58 5.963v4.353a1.226 1.226 0 1 1-1.008-.036V5.887a1.226 1.226 0 0 1-.666-1.608L5.093 2.465l-4.79 4.79a1.03 1.03 0 0 0 0 1.457l6.986 6.986a1.03 1.03 0 0 0 1.457 0l6.953-6.953a1.03 1.03 0 0 0 0-1.457"
                    }
                }
                div {
                    strong { "Open Source Notice: " }
                    "Project Echelon is open source software licensed under the MIT License. You can review our code at "
                    a {
                        href: "https://github.com/COTFK/project-echelon",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        class: "link link-info",
                        "our Forgejo repository"
                    }
                    "."
                }
            }
        }
    }
}
