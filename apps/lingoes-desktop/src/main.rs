use std::time::Duration;

fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        if let Ok(exe) = std::env::current_exe() {
            if let Ok(data_root) = lingoes_desktop::resolve_app_data_root(&exe) {
                let endpoint =
                    lingoes_platform::ipc::LocalEndpoint::new(&data_root.path, "lexiforge");
                match lingoes_platform::ipc::InstanceLock::acquire(endpoint.root(), endpoint.name())
                {
                    Ok(lingoes_platform::ipc::InstanceLock::Primary(_guard)) => {
                        if let Err(error) = lingoes_desktop::launch_gui() {
                            eprintln!("{error}");
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
        if let Err(error) = lingoes_desktop::launch_gui() {
            eprintln!("{error}");
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
