//! Navigation bar component.

use dioxus::prelude::*;
use crate::types::DISCORD_INVITE_URL;

/// Application navigation bar.
#[component]
pub fn NavBar() -> Element {
    rsx! {
        div {
            class: "drawer",
            input {
                id: "echelon-menu",
                r#type: "checkbox",
                class: "drawer-toggle"
            }
            div {
                class: "drawer-content flex flex-col",
                nav {
                    class: "navbar sticky flex bg-base-300 shadow-sm justify-between",
                    a {
                        href: "/",
                        class: "btn btn-ghost text-xl",
                        img {
                            class: "size-8",
                            src: asset!("/assets/logo.png")
                        }
                        "Project Echelon"
                    }
                    div {
                        class: "flex-none sm:hidden px-4",
                        label {
                            r#for: "echelon-menu",
                            aria_label: "Open menu",
                            class: "btn btn-square btn-ghost",
                            svg {
                                xmlns: "http://www.w3.org/2000/svg",
                                fill: "none",
                                view_box: "0 0 24 24",
                                class: "inline-block h-6 w-6 stroke-current",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M4 6h16M4 12h16M4 18h16"
                                }
                            }
                        }
                    }
                    div {
                        class: "hidden flex-none sm:block",
                        ul {
                            class:"menu menu-md menu-horizontal text-base-content/90",
                            Menu {}
                        }
                    }
                }
            }
            div {
                class: "drawer-side w-full",
                ul {
                    class: "menu menu-lg bg-base-200 min-h-full w-80 pl-8 w-full max-w-128",
                    label {
                        r#for: "echelon-menu",
                        aria_label: "Close menu", 
                        class: "btn btn-ghost text-3xl w-full flex justify-end pt-8",
                        svg {
                            xmlns: "http://www.w3.org/2000/svg", 
                            class: "size-8",
                            fill: "currentColor", 
                            view_box: "0 0 16 16",
                            path {
                                d: "M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708"
                            }
                        }
                    }
                    div {
                        class: "w-fit",
                        Menu {}
                    }
                }
            }
        }
    }
}

#[component]
fn Menu() -> Element {
    rsx!(
        li {
            a {
                href: DISCORD_INVITE_URL,
                target: "_blank",
                svg {
                    xmlns: "http://www.w3.org/2000/svg",
                    width: "1em",
                    height: "1em",
                    fill: "currentColor",
                    class: "text-2xl mr-2 md:text-lg md:mr-1",
                    view_box: "0 0 16 16",
                    path {
                        d: "M13.545 2.907a13.2 13.2 0 0 0-3.257-1.011.05.05 0 0 0-.052.025c-.141.25-.297.577-.406.833a12.2 12.2 0 0 0-3.658 0 8 8 0 0 0-.412-.833.05.05 0 0 0-.052-.025c-1.125.194-2.22.534-3.257 1.011a.04.04 0 0 0-.021.018C.356 6.024-.213 9.047.066 12.032q.003.022.021.037a13.3 13.3 0 0 0 3.995 2.02.05.05 0 0 0 .056-.019q.463-.63.818-1.329a.05.05 0 0 0-.01-.059l-.018-.011a9 9 0 0 1-1.248-.595.05.05 0 0 1-.02-.066l.015-.019q.127-.095.248-.195a.05.05 0 0 1 .051-.007c2.619 1.196 5.454 1.196 8.041 0a.05.05 0 0 1 .053.007q.121.1.248.195a.05.05 0 0 1-.004.085 8 8 0 0 1-1.249.594.05.05 0 0 0-.03.03.05.05 0 0 0 .003.041c.24.465.515.909.817 1.329a.05.05 0 0 0 .056.019 13.2 13.2 0 0 0 4.001-2.02.05.05 0 0 0 .021-.037c.334-3.451-.559-6.449-2.366-9.106a.03.03 0 0 0-.02-.019m-8.198 7.307c-.789 0-1.438-.724-1.438-1.612s.637-1.613 1.438-1.613c.807 0 1.45.73 1.438 1.613 0 .888-.637 1.612-1.438 1.612m5.316 0c-.788 0-1.438-.724-1.438-1.612s.637-1.613 1.438-1.613c.807 0 1.451.73 1.438 1.613 0 .888-.631 1.612-1.438 1.612"
                    }
                }
                "Discord Bot",
            }
        }
        li {
            a {
                href: "/help",
                svg {
                    xmlns: "http://www.w3.org/2000/svg", 
                    width: "1em",
                    height: "1em",
                    fill: "currentColor",
                    class: "text-2xl mr-2 md:text-lg md:mr-0",
                    view_box: "0 0 16 16",
                    path {
                        fill_rule: "evenodd",
                        d:"M4.475 5.458c-.284 0-.514-.237-.47-.517C4.28 3.24 5.576 2 7.825 2c2.25 0 3.767 1.36 3.767 3.215 0 1.344-.665 2.288-1.79 2.973-1.1.659-1.414 1.118-1.414 2.01v.03a.5.5 0 0 1-.5.5h-.77a.5.5 0 0 1-.5-.495l-.003-.2c-.043-1.221.477-2.001 1.645-2.712 1.03-.632 1.397-1.135 1.397-2.028 0-.979-.758-1.698-1.926-1.698-1.009 0-1.71.529-1.938 1.402-.066.254-.278.461-.54.461h-.777ZM7.496 14c.622 0 1.095-.474 1.095-1.09 0-.618-.473-1.092-1.095-1.092-.606 0-1.087.474-1.087 1.091S6.89 14 7.496 14"
                    }
                }
                "Help",
            }
        }
    )
}