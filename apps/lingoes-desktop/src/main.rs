#![cfg_attr(windows, windows_subsystem = "windows")]

use std::time::Duration;

#[cfg(windows)]
fn attach_parent_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    unsafe {
        // If launched from PowerShell or Command Prompt, attach to display stdout/stderr
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

fn main() {
    #[cfg(windows)]
    attach_parent_console();

    lingoes_desktop::setup_panic_hook();
    lingoes_desktop::log_startup("=== Lingoes RS Application Started ===");

    let arguments: Vec<_> = std::env::args().skip(1).collect();

    // Check for explicit software renderer flag
    if arguments.iter().any(|a| a == "--software" || a == "--soft") {
        unsafe {
            std::env::set_var("SLINT_BACKEND", "winit-software");
        }
        lingoes_desktop::log_startup("Forced software renderer via command-line argument");
    }

    if arguments.is_empty()
        || arguments[0] == "gui"
        || arguments[0] == "--gui"
        || arguments[0] == "--software"
        || arguments[0] == "--soft"
    {
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
                        lingoes_desktop::log_startup("Primary single-instance lock acquired");
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
                        lingoes_desktop::log_startup(
                            "Secondary instance detected. Attempting to activate primary instance...",
                        );
                        let send_res = endpoint.send(
                            &lingoes_platform::ipc::InstanceCommand::Activate,
                            Duration::from_secs(2),
                        );
                        if let Err(err) = send_res {
                            lingoes_desktop::log_startup(&format!(
                                "Cannot activate primary instance ({err}). Primary may be stale/unreachable. Launching GUI directly..."
                            ));
                            if let Err(error) = lingoes_desktop::launch_gui_with_options(
                                initial_query,
                                initial_popup,
                            ) {
                                lingoes_desktop::show_fatal_error(
                                    "Lingoes RS - Startup Error",
                                    &error.to_string(),
                                );
                                std::process::exit(2);
                            }
                        } else {
                            lingoes_desktop::log_startup(
                                "Successfully forwarded activate command to existing instance. Exiting secondary.",
                            );
                        }
                        return;
                    }
                    Err(error) => {
                        lingoes_desktop::log_startup(&format!(
                            "Notice: could not acquire single-instance lock ({error}), proceeding"
                        ));
                    }
                }
            }
        }
        if let Err(error) = lingoes_desktop::launch_gui_with_options(initial_query, initial_popup) {
            lingoes_desktop::show_fatal_error("Lingoes RS - Startup Error", &error.to_string());
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
