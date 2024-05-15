use dioxus::prelude::*;

#[component]
pub fn StyleMainWindow() -> Element {
    let text = r#"
.main-window {
    width: 100%;
    height: 100%;
    background-color: black;
}
    "#;

    rsx! {
        style { { text} }
    }
}

/// A component that defines the style for a button.
///
/// Classes
/// - `rbutton`
/// - `gbutton`
#[component]
pub fn StyleButton(dark_mode: bool) -> Element {
    #[component]
    pub fn gen_button_style(
        class: String,
        color: String,
        color_hover: String,
        color_click: String,
    ) -> Element {
        rsx! { "
.{class} {{
    background-color: {color};
    border: none;
}}
.{class}:hover {{
    background-color: {color_hover};
}}
.{class}:active {{
    background-color: {color_click};
}}" 
        }
    }

    rsx! {
        style {
            gen_button_style { class: "gbutton", color: "#25a750", color_hover: "#258d47", color_click: "#25a750" }
            gen_button_style { class: "rbutton", color: "#ca3f64", color_hover: "#a93957", color_click: "#ca3f64" }
        }
    }
}

#[component]
pub fn StyleFont() -> Element {
    let text = format!(".font1 {{ font-family: 'Open Sans', sans-serif; }}");
    let text = (1..=100).fold(text, |acc, i| {
        format!("{}\n.font-size-{} {{ font-size: {}px; }}", acc, i, i)
    });

    // Append font color
    let text = format!("{}\n.font-color-w {{ color: white; }}", text);
    let text = format!("{}\n.font-color-b {{ color: black; }}", text);

    rsx! {
        link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        link { rel: "preconnect", href: "https://fonts.gstatic.com" }
        link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Open+Sans&display=swap"
        }

        style { { text } }
    }
}

#[component]
pub fn StyleColor() -> Element {
    let text = r#"
.color-0 { background-color: #000000; }
.color-1 { background-color: #121212; }
.color-2 { background-color: #202020; }
.color-3 { background-color: #505050; }
.color-4 { background-color: #808080; }
.color-5 { background-color: #ffffff; }
    "#;

    rsx! {
        style { { text } }
    }
}