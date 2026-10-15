#![cfg_attr(windows, windows_subsystem = "windows")]

use std::time::Duration;

fn main() {
    lingoes_desktop::setup_panic_hook();
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.is_empty() || arguments[0] == "gui" || arguments[0] == "--gui" {
        let initial_query =
            if !arguments.is_empty() && (arguments[0] == "gui" || arguments[0] == "--gui") {
                arguments.get(1).map(String::as_str)
            } else {
                None
            };
        let initial_popup =
            if !arguments.is_empty() && (arguments[0] == "gui" || arguments[0] == "--gui") {
                arguments.get(2).map(String::as_str)
            } else {
                None
            };
        if let Ok(exe) = std::env::current_exe() {
            if let Ok(data_root) = lingoes_desktop::resolve_app_data_root(&exe) {
                let endpoint =
                    lingoes_platform::ipc::LocalEndpoint::new(&data_root.path, "lexiforge");
                match lingoes_platform::ipc::InstanceLock::acquire(endpoint.root(), endpoint.name())
                {
                    Ok(lingoes_platform::ipc::InstanceLock::Primary(_guard)) => {
                        if let Err(error) =
                            lingoes_desktop::launch_gui_with_options(initial_query, initial_popup)
                        {
                            lingoes_desktop::show_fatal_error(
                                "Lingoes RS - Startup Error",
                                &error.to_string(),
                            );
                            std::process::exit(2);
                        }
                        return;
                    }
                    Ok(lingoes_platform::ipc::InstanceLock::Secondary) => {
                        let _ = endpoint.send(
                            &lingoes_platform::ipc::InstanceCommand::Activate,
                            Duration::from_secs(2),
                        );
                        return;
                    }
                    Err(error) => {
                        eprintln!(
                            "Notice: could not acquire single-instance lock ({error}), proceeding"
                        );
                    }
                }
            }
        }
        if let Err(error) = lingoes_desktop::launch_gui_with_options(initial_query, initial_popup) {
            lingoes_desktop::show_fatal_error(
                "Lingoes RS - Startup Error",
                &error.to_string(),
            );
            std::process::exit(2);
        }
    } else {
        match lingoes_desktop::run(arguments) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }
}
