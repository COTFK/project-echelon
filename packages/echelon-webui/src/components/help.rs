use dioxus::prelude::*;
use crate::types::DISCORD_INVITE_URL;

#[component]
pub fn WebsiteHelp() -> Element {
    rsx!(
        h2 { class: "text-xl font-semibold mb-4", "How does Echelon work?" }
        p { "Project Echelon opens your replay in a custom instance of EDOPro and records the gameplay directly to an x264 MP4 video file." }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "Why do I sometimes have to wait in a queue?" }
        p { "Replays are recorded one by one, on a first-come, first-serve basis, to ensure the final videos are smooth and high-quality." } 

        h2 { class: "text-xl font-semibold mt-8 mb-4", "How long do you store the final videos?" }
        p { 
            "Recorded replays are stored for 1 hour only, so download them as soon as possible. For more information, read our " 
            a { 
                class: "underline",
                href: "/privacy",
                target: "_blank",
                "Privacy Policy" 
            }
            "."
        }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "My replay is from YGO Omega / TDOANE / another simulator - can I use it with Echelon?" }
        p { "No - only " span { class: "font-mono bg-base-content/20", ".yrpX"} " files from Project Ignis: EDOPro are supported. Other simulators use incompatible formats that Echelon cannot open." }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "Do you support custom cards?" }
        p { "No, Echelon uses the latest official card database provided by Project Ignis; as such, replays using custom cards will either not work with Echelon or cause it to behave in unexpected ways." }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "What's the trade-off between the video presets?" }
        p { "'File-size optimized' produces smaller files (easier to share on Discord and other platforms with a file size limit). " } 
        p { "'Balanced' has average quality and file-size, and is the recommended preset for most uses." }
        p { "'High quality' produces sharper video but files can be 2-5x larger, so only use it if you're uploading to YouTube or archiving." } 

        h2 { class: "text-xl font-semibold mt-8 mb-4", "My replay has been 'processing' for a very long time. What's going on?"}
        p { "At default settings, Echelon takes roughly the same time to record as EDOPro takes to play it, so a 10-minute replay will take 10 minutes to record." }
        p { "Using advanced settings, like a different quality and/or game speed, does affect the processing time - recording at 0.5x speed will take twice as long as recording at 1x speed." }
        p { "If your replay has been processing for much longer than twice its run time in EDOPro, contact us to investigate."}

        h2 { class: "text-xl font-semibold mt-8 mb-4", "I get an 'Invalid replay file' error, but I'm sure my file is valid." }
        p {
            "Make sure your replay comes from a recent version of EDOPro; Project Echelon was tested with replays produced from EDOPro 40 'Puppet of Strings' and EDOPro 41 'Bagooska'."
        }
        p {"Additionally, we do not support replays coming from EDOPro forks, or other simulators based on YGOPro." }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "Is Project Echelon open-source?" }
        p { 
            "Yes! Project Echelon is MIT-licensed. "
            "The source code for both Echelon and the custom fork of EDOPro (AGPLv3 licensed) used by it are available on "
            a { 
                class: "underline",
                href: "https://git.arqalite.org/COTFK/project-echelon",
                target: "_blank",
                "our Forgejo instance" 
            }
            "."
        }


        Contact {}
    )
}

#[component]
pub fn DiscordBotHelp() -> Element {
    rsx!(
        h2 { class: "text-xl font-semibold mb-4", "How do I add the bot to my server?"}
        p { 
            "You can use " 
            a { 
                class: "underline",
                href: DISCORD_INVITE_URL,
                target: "_blank",
                "this install link" 
            } 
            " to get started." 
        }
        p { "Discord will ask which server you want to add it to, and what permissions the bot should have - normally only 'Send Messages' is required."}

        h2 { class: "text-xl font-semibold mt-8 mb-4", "How do I use the bot?" }
        p { "Type " span { class: "font-mono bg-base-content/20", "/echelon convert"} " to invoke the command and upload a replay file. Send the message to start converting."}

        h2 { class: "text-xl font-semibold mt-8 mb-4", "Can I send multiple replays at once?" }
        p { "No, each replay has to be sent in a separate message. However you can call the command as many times as you need!" }
    
        h2 { class: "text-xl font-semibold mt-8 mb-4", "Can I configure the video quality and game speed?" }
        p { "No, the bot will always use the default settings when recording replays." }
        p { "For more control over your replays, use the website." }

        h2 { class: "text-xl font-semibold mt-8 mb-4", "Why did I receive a download link instead of a video?" }
        p { "Discord has a 10MB file size limit, so if the bot cannot upload the file to Discord directly, it will instead send you a download link." }
        p { "Keep in mind that download links are valid for 1 hour only - so download your video as soon as possible!"}

        h2 { class: "text-xl font-semibold mt-8 mb-4", "The bot is online, but it is not responding to my commands." }
        p { "Make sure the bot has the Send Messages permission - if needed, kick Echelon out of the server and add it back again." }
        p { "If the issue persists, contact us and we'll investigate."}

        h2 { class: "text-xl font-semibold mt-8 mb-4", "The bot acknowledged my file but never sent the video." }
        p { 
            "The bot should send the video (or download link, for videos larger than 10MB) in the same channel where you ran the command. "
            "Make sure the bot has permissions to send messages in that channel." 
        }
        p { "If the issue persists, contact us and we'll investigate."}

        Contact {}

    )
}

#[component]
fn Contact() -> Element {
    rsx!(
        h2 { class: "text-xl font-semibold mt-8 mb-4", "How can I contact you?" }
        p { 
            "The best way to get help is to join the "
            a { 
                class: "underline",
                href: "https://discord.gg/8JtxHUAdGq",
                target: "_blank",
                "Circle of the Fire Kings" 
            }
            " Discord server and ask your question in the "
            span {
                class: "font-mono bg-base-content/20",
                "#echelon-discussion"
            }
            " forum."
        }
        p {
            "If that isn't an option, feel free to send an email to "
            a {
                class: "underline",
                href: "mailto:feedback@arqalite.org",
                "feedback@arqalite.org" 
            }
            " instead. We'll reply as soon as possible!"
        }
    )
}