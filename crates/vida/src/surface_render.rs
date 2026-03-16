use crate::RenderMode;

pub(crate) fn print_surface_header(render: RenderMode, title: &str) {
    match render {
        RenderMode::Plain => println!("{title}"),
        RenderMode::Color => println!("\x1b[1;36m{title}\x1b[0m"),
        RenderMode::ColorEmoji => println!("\x1b[1;36m📘 {title}\x1b[0m"),
    }
}

pub(crate) fn print_surface_line(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: {value}"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: {value}"),
        RenderMode::ColorEmoji => println!("🔹 \x1b[1;34m{label}\x1b[0m: {value}"),
    }
}

pub(crate) fn print_surface_ok(render: RenderMode, label: &str, value: &str) {
    match render {
        RenderMode::Plain => println!("{label}: ok ({value})"),
        RenderMode::Color => println!("\x1b[1;34m{label}\x1b[0m: \x1b[1;32mok\x1b[0m ({value})"),
        RenderMode::ColorEmoji => {
            println!("✅ \x1b[1;34m{label}\x1b[0m: \x1b[1;32mok\x1b[0m ({value})")
        }
    }
}
