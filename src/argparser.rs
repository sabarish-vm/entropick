use clap::{Arg, ArgAction, ArgMatches, Command};
use secrecy::ExposeSecret;

use crate::generator::generate_from_pattern;
use crate::profiles::{
    add_profile, edit_profile, get_profile, list_profiles, remove_profile, search_profiles,
};
use crate::types::Profile;

pub fn build_parser() -> Command {
    Command::new("entropick")
        .author("Sabarish, github.com/sabarish-vm")
        .about("Password generator : Generate conveniently from saved patterns")
        // ---------- generate ----------
        .subcommand(
            Command::new("generate")
                .about("Generate a password from a pattern or saved profile")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("pattern")
                        .short('p')
                        .long("pattern")
                        .help("The pattern string (e.g. [[a-z]][[A-Z]][[0-9]]((!@#$%^&*)))"),
                )
                .arg(
                    Arg::new("length")
                        .value_parser(clap::value_parser!(usize))
                        .short('l')
                        .long("length")
                        .help("Password length (overrides profile's stored length)")
                        .default_value("16"),
                )
                .arg(
                    Arg::new("profile")
                        .long("profile")
                        .help("Name of a saved profile to use for generation"),
                )
                .arg(
                    Arg::new("count")
                        .value_parser(clap::value_parser!(usize))
                        .short('c')
                        .long("count")
                        .help("Number of passwords to generate")
                        .default_value("1"),
                ),
        )
        // ---------- profile ----------
        .subcommand(
            Command::new("profile")
                .about("Manage saved password profiles")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("add")
                        .about("Add a new profile")
                        .arg(
                            Arg::new("name")
                                .action(ArgAction::Set)
                                .required(true)
                                .help("Name for the profile"),
                        )
                        .arg(
                            Arg::new("pattern")
                                .short('p')
                                .long("pattern")
                                .required(true)
                                .help("The pattern string for this profile"),
                        )
                        .arg(
                            Arg::new("length")
                                .value_parser(clap::value_parser!(usize))
                                .short('l')
                                .long("length")
                                .required(true)
                                .help("Password length to store in the profile"),
                        ),
                )
                .subcommand(Command::new("list").about("List all saved profiles"))
                .subcommand(
                    Command::new("show").about("Show a profile's details").arg(
                        Arg::new("name")
                            .action(ArgAction::Set)
                            .required(true)
                            .help("Name of the profile to show"),
                    ),
                )
                .subcommand(
                    Command::new("edit")
                        .about("Edit an existing profile")
                        .arg(
                            Arg::new("name")
                                .action(ArgAction::Set)
                                .required(true)
                                .help("Name of the profile to edit"),
                        )
                        .arg(
                            Arg::new("pattern")
                                .short('p')
                                .long("pattern")
                                .help("New pattern string"),
                        )
                        .arg(
                            Arg::new("length")
                                .short('l')
                                .long("length")
                                .help("New password length"),
                        ),
                )
                .subcommand(
                    Command::new("remove").about("Remove a profile").arg(
                        Arg::new("name")
                            .action(ArgAction::Set)
                            .required(true)
                            .help("Name of the profile to remove"),
                    ),
                )
                .subcommand(
                    Command::new("search").about("Search profiles by name").arg(
                        Arg::new("query")
                            .action(ArgAction::Set)
                            .required(true)
                            .help("Search query (case-insensitive)"),
                    ),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell autocompletion scripts")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("shell")
                        .action(ArgAction::Set)
                        .required(true)
                        .value_parser(clap::value_parser!(clap_complete::Shell)), // 🔒 Restricts inputs strictly to valid shells
                ),
        )
}

/// Parse CLI arguments and dispatch to the appropriate handler.
pub(crate) fn run() {
    let mut parser = build_parser();
    let matches = parser.get_matches_mut();

    match matches.subcommand() {
        Some(("generate", generate_matches)) => {
            handle_generate(generate_matches);
        }
        Some(("profile", profile_matches)) => {
            handle_profile(profile_matches);
        }
        Some(("completions", sub_matches)) => {
            handle_completions(sub_matches);
        }
        _ => {
            let _ = parser.print_help();
        }
    }
}

fn handle_generate(matches: &ArgMatches) {
    let profile = matches.get_one::<String>("profile");
    let pattern = matches.get_one::<String>("pattern");
    let length = matches.get_one::<usize>("length");
    let count = *matches.get_one::<usize>("count").unwrap_or(&1);

    // Resolve pattern and length from profile, pattern flag, or defaults
    let (pat, len): (String, usize) = if let Some(name) = profile {
        let prof = get_profile(name);
        if prof.is_none() {
            eprintln!("profile '{name}' not found");
            std::process::exit(1);
        }
        let p = prof.unwrap();
        if let Some(_len) = length {
            (p.pattern, *_len)
        } else {
            (p.pattern, p.length)
        }
    } else if let Some(p) = pattern {
        (p.clone(), *length.unwrap_or(&16usize))
    } else {
        eprintln!("either --pattern or --profile is required");
        std::process::exit(1)
    };

    println!("\nPassword : log10(guesses), zxcvbn-score\n");
    for _ in 0..count {
        let result = generate_from_pattern(&pat, len);
        if let Err(err) = result {
            eprintln!("Error {}", err);
            std::process::exit(1);
        }
        let genp = result.unwrap();
        let s = genp.value.expose_secret();
        let ent_bits = genp.entropy.guesses_log10();
        let zscore = genp.entropy.score();
        println!("{s} : {ent_bits:.2}, {zscore}");
    }
}

fn handle_profile(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("add", add_matches)) => {
            let name = add_matches.get_one::<String>("name").unwrap();
            let pattern = add_matches.get_one::<String>("pattern").unwrap();
            let length = add_matches.get_one::<usize>("length").unwrap();

            let profile = Profile {
                pattern: pattern.clone(),
                length: *length,
            };
            let result = add_profile(name, profile);
            if let Err(res) = result {
                eprintln!("error: {}", res);
                std::process::exit(1);
            }
            println!("profile '{name}' added");
        }
        Some(("list", _)) => {
            let names = list_profiles();
            if names.is_empty() {
                println!("no profiles saved");
                return;
            }
            for name in names {
                println!("  {name}");
            }
        }
        Some(("show", show_matches)) => {
            let name = show_matches.get_one::<String>("name").unwrap();
            let profile = get_profile(name);
            if let Some(p) = profile {
                println!("profile: {name}");
                println!("  pattern: {}", p.pattern);
                println!("  length:  {}", p.length);
            } else {
                eprintln!("profile '{name}' not found");
                std::process::exit(1);
            }
        }
        Some(("edit", edit_matches)) => {
            let name = edit_matches.get_one::<String>("name").unwrap();
            let pattern = edit_matches.get_one::<String>("pattern");
            let length = edit_matches.get_one::<usize>("length");

            let result = edit_profile(name, pattern, length);
            if let Err(err) = result {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
            println!("profile '{name}' updated");
        }
        Some(("remove", rm_matches)) => {
            let name = rm_matches.get_one::<String>("name").unwrap();
            let result = remove_profile(name);
            if let Err(err) = result {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
            println!("profile '{name}' removed");
        }
        Some(("search", search_matches)) => {
            let query = search_matches.get_one::<String>("query").unwrap();
            let results = search_profiles(query);
            if results.is_empty() {
                println!("no profiles match '{query}'");
                return;
            }
            for (name, profile) in results {
                println!("  {name}: {} chars", profile.length);
            }
        }
        Some((x, _)) => {
            eprintln!("unknown subcommand for profile: {}", x);
            std::process::exit(1);
        }
        None => {
            eprintln!("No additional args/commands provided for the profile subcommand");
            std::process::exit(1);
        }
    }
}

fn handle_completions(matches: &ArgMatches) {
    let shell = matches.get_one::<clap_complete::Shell>("shell").unwrap();
    let mut cmd = build_parser();
    let bin_name = cmd.get_name().to_string();

    // Generates the completion script structure and dumps it straight to stdout
    clap_complete::generate(*shell, &mut cmd, bin_name, &mut std::io::stdout());
}
