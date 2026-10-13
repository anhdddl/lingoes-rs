fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let result = if arguments.is_empty() {
        lingoes_desktop::launch_gui().map(|()| None)
    } else {
        lingoes_desktop::run(arguments).map(Some)
    };

    match result {
        Ok(Some(output)) => println!("{output}"),
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
