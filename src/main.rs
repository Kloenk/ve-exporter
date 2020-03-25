#[macro_use]
extern crate log;
extern crate env_logger;
use serde_json::Value;
use clap::{App, Arg, SubCommand};

use ve_exporter::Config;
use futures::FutureExt;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        println!("set env var");
        std::env::set_var("RUST_LOG", "actix_web=info,ve_exporter=trace");
    }
    env_logger::init();
    let mut app = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .setting(clap::AppSettings::ColorAuto)
        .setting(clap::AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("set config file")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .value_name("PORT")
                .help("port for the http server")
                .takes_value(true)
            //.default_value("9701")
        )
        .arg(
            Arg::with_name("address")
                .long("address")
                .value_name("ADDRESS")
                .help("listening address")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("timeout")
                .long("timeout")
                .short("t")
                .value_name("SECONDS")
                .help("set timeout for serial connection")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("device")
                .long("serial")
                .short("serial")
                .value_name("DEVICE")
                .help("address of the serial2tcp")
                .takes_value(true)
        );

    if cfg!(feature = "completion") {
        app = app.subcommand(
            SubCommand::with_name("completion")
                .about("create completions")
                .version("0.1.0")
                .author(env!("CARGO_PKG_AUTHORS"))
                .arg(
                    Arg::with_name("shell")
                        .help("set the shell to create for. Tries to identify with env variable")
                        .index(1)
                        .required(false)
                        .value_name("SHELL")
                        .possible_value("fish")
                        .possible_value("bash")
                        .possible_value("zsh")
                        .possible_value("powershell")
                        .possible_value("elvish"),
                )
                .arg(
                    Arg::with_name("out")
                        .help("sets output file")
                        .value_name("FILE")
                        .short("o")
                        .long("output"),
                )
                .setting(clap::AppSettings::ColorAuto)
                .setting(clap::AppSettings::ColoredHelp)
        );
    }

    let matches = app.clone().get_matches();

    if cfg!(feature = "completion") {
        if let Some(matches) = matches.subcommand_matches("completion") {
            trace!("generate completion");
            completion(&matches, &mut app);
            std::process::exit(0);
        }
    }
    drop(app);

    let mut config = Config::new();

    let j_config: Option<serde_json::Value> = matches.value_of("config")
            .map(|v| std::fs::File::open(v).ok()
            .map(|v| serde_json::from_reader(std::io::BufReader::new(v)).ok())).flatten().flatten();

    if let Some(value) = &matches.value_of("address") {
            config.address = value.to_string();
    } else if let Some(value) = j_config.as_ref().map(|v| v.get("address")).flatten().map(|v| v.as_str()).flatten() {
        config.address = value.to_string();
    }

    if let Some(value) = matches.value_of("port").map(|v| v.parse::<u16>().ok()).flatten() {
        trace!("set port to {}", value);
        config.port = value;
    } else if let Some(value) = j_config.as_ref().map(|v| v.get("port").map(|v| v.as_i64().map(|v| v as u16))).flatten().flatten() {
        trace!("set port to {}", value);
        config.port = value;
    }

    if let Some(value) = matches.value_of("device") {
        trace!("use {} for serial connection", value);
        config.device = value.to_string();
    } else if let Some(value) = j_config.as_ref().map(|v| v.get("device").map(|v| v.as_str())).flatten().flatten() {
        trace!("use {} for serial connection", value);
        config.device = value.to_string();
    }

    if let Some(value) = matches.value_of("timeout").map(|v| v.parse::<u64>().ok()).flatten() {
        trace!("set a timeout of {} seconds", value);
        config.timeout = std::time::Duration::from_secs(value);
    } else if let Some(value) = j_config.as_ref().map(|v| v.get("timeout").map(|v| v.as_u64())).flatten().flatten() {
        trace!("set a timeout of {} seconds", value);
        config.timeout = std::time::Duration::from_secs(value);
    }

    drop(matches);
    drop(j_config);

    config.run().await.unwrap();

    unreachable!()
}

/// create completion
#[cfg(feature = "completion")]
fn completion(args: &clap::ArgMatches, app: &mut App) {
    let shell: String = match args.value_of("shell") {
        Some(shell) => shell.to_string(),
        None => shell(),
    };

    use clap::Shell;
    let shell_l = shell.to_lowercase();
    let shell: Shell;
    if shell_l == "fish" {
        shell = Shell::Fish;
    } else if shell_l == "zsh" {
        shell = Shell::Zsh;
    } else if shell_l == "powershell" {
        shell = Shell::PowerShell;
    } else if shell_l == "elvish" {
        shell = Shell::Elvish;
    } else {
        shell = Shell::Bash;
    }

    use std::fs::File;
    use std::io::BufWriter;
    use std::io::Write;

    let mut path = BufWriter::new(match args.value_of("out") {
        Some(x) => Box::new(
            File::create(&std::path::Path::new(x)).unwrap_or_else(|err| {
                eprintln!("Error opening file: {}", err);
                std::process::exit(1);
            }),
        ) as Box<dyn Write>,
        None => Box::new(std::io::stdout()) as Box<dyn Write>,
    });

    app.gen_completions_to(env!("CARGO_PKG_NAME"), shell, &mut path);
}

#[cfg(all(feature = "completion", not(windows)))]
fn shell() -> String {
    let shell: String = match std::env::var("SHELL") {
        Ok(shell) => shell,
        Err(_) => "/bin/bash".to_string(),
    };
    let shell = std::path::Path::new(&shell);
    match shell.file_name() {
        Some(shell) => shell.to_os_string().to_string_lossy().to_string(),
        None => "bash".to_string(),
    }
}

#[cfg(all(feature = "completion", windows))]
fn shell() -> String {
    "powershell".to_string() // always default to powershell on windows
}

#[cfg(not(feature = "completion"))]
fn completion(_: &clap::ArgMatches, _: &mut App) {
    eprintln!("Completion command fired but completion not included in features");
    std::process::exit(-1);
}
