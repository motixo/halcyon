mod app;
mod automation;
mod calendar;
mod config;
mod data;
mod desktop;
mod desktop_theme;
mod drawer;
mod layout;
mod layout_state;
mod materials;
#[cfg(test)]
mod native_test;
mod paths;
mod schedule;
mod settings;
mod system_theme;
mod tools;
mod usage;
mod watch;
mod widget_style;

fn main() -> cosmic::iced::Result {
    // Settings from before the rename move to the new folders, once.
    paths::migrate();
    let mut args = std::env::args().skip(1);
    let command = args.next();
    if matches!(
        command.as_deref(),
        Some("--select-palette" | "--set-cosmic-colors")
    ) {
        let value = args.next().unwrap_or_default();
        let request = match (command.as_deref(), value.as_str()) {
            (Some("--select-palette"), name) => Ok(system_theme::ControllerRequest::Palette(name)),
            (_, "on") => Ok(system_theme::ControllerRequest::CosmicColors(true)),
            (_, "off") => Ok(system_theme::ControllerRequest::CosmicColors(false)),
            _ => Err("Choose on or off".to_string()),
        };
        let result = request.and_then(system_theme::controller);
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if command.as_deref() == Some("--apply-cosmic-colors") {
        let result = args
            .next()
            .ok_or_else(|| "Choose a Halcyon palette".into())
            .and_then(|name| system_theme::apply(&name));
        match result {
            Ok(()) => println!("COSMIC accents matched"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if command.as_deref() == Some("--apply-palette-links") {
        let result = args
            .next()
            .ok_or_else(|| "Choose a Halcyon palette".to_string())
            .and_then(|name| {
                let mut links = system_theme::Links::default();
                for flag in args.by_ref() {
                    match flag.as_str() {
                        "--wallpaper" => links.wallpaper = true,
                        "--colors" => links.colors = true,
                        other => return Err(format!("Unknown option {other}")),
                    }
                }
                if !links.wallpaper && !links.colors {
                    return Err("Choose --wallpaper, --colors or both".into());
                }
                system_theme::apply_links(&name, links)
            });
        match result {
            Ok(()) => println!("Palette matched"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if command.as_deref() == Some("--restore-background") {
        let result = args
            .next()
            .ok_or_else(|| "Choose the saved wallpaper file".to_string())
            .and_then(|path| system_theme::restore_background(std::path::Path::new(&path)));
        match result {
            Ok(()) => println!("Previous wallpaper restored"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if command.as_deref() == Some("--settings") {
        return settings::run(args.next().as_deref().unwrap_or(""));
    }
    usage::start(paths::state_dir().map(|dir| dir.join("usage.json")));
    let settings = cosmic::app::Settings::default()
        .no_main_window(true)
        .client_decorations(false)
        .transparent(true);
    cosmic::app::run::<app::Cards>(settings, config::Config::load())
}
