use crate::config;

pub fn run() {
    println!("Initializing new Cortex project...");

    if config::load().is_ok() {
        println!("Cortex.toml already exists, aborting.");
    } else {
        let res = config::save_default();
        match res {
            Ok(()) => println!("Cortex configuration written successfully."),
            Err(e) => eprintln!("Failed to write Cortex configuration: {:#?}", e),
        }
    }
}
