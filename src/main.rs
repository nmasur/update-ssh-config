use anyhow::{anyhow, Context, Result};
use clap::{crate_version, App, Arg};
use dirs_next::home_dir;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let matches = App::new("update-ssh-config")
        .version(crate_version!())
        .author("Noah Masur <noahmasur@gmail.com>")
        .about("Updates ~/.ssh/config file hostname")
        .arg(
            Arg::with_name("host")
                .required(true)
                .short("h")
                .long("host")
                .value_name("HOST")
                .help("Name of host in config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("hostname")
                .required(true)
                .short("n")
                .long("hostname")
                .value_name("HOSTNAME")
                .help("New hostname to replace current one")
                .takes_value(true),
        )
        .get_matches();

    update_config(
        matches.value_of("host").unwrap(),
        matches.value_of("hostname").unwrap(),
    )
}

fn update_config(host: &str, new_hostname: &str) -> Result<()> {
    let config_path =
        hardcoded_config_location().context("Failed to find path for config file.")?;
    let config_lines = read_config_file(&config_path).context("Problem reading config file.")?;
    let mut split_lines = split_lines_on_host(config_lines, &host)?;
    if split_lines.hostname != new_hostname {
        println!(
            "Modifying {} hostname from '{}' to '{}'.",
            &host, &split_lines.hostname, &new_hostname
        );
        split_lines.hostname = String::from(new_hostname);
        rewrite_config_file(&config_path, split_lines)
            .context("Error while writing new config file.")?;
        println!("Done.")
    }

    Ok(())
}

fn hardcoded_config_location() -> Result<PathBuf> {
    match home_dir() {
        Some(mut home_path) => {
            home_path.push(".ssh");
            home_path.push("config");
            Ok(home_path)
        }
        None => {
            eprintln!("Failed to determine home directory.");
            std::process::exit(1);
        }
    }
}

fn read_config_file(file_path: &PathBuf) -> std::io::Result<Vec<String>> {
    let file = File::open(file_path.as_path())?;
    let buf_reader = BufReader::new(file);
    let lines = buf_reader
        .lines()
        .map(|line| line.unwrap_or_else(|_| String::from("")))
        .collect();
    Ok(lines)
}

struct SplitLines {
    before: Vec<String>,
    after: Vec<String>,
    host: String,
    hostname: String,
    user: String,
    identityfile: String,
}

impl SplitLines {
    fn new(host: &str) -> Self {
        SplitLines {
            before: Vec::new(),
            after: Vec::new(),
            host: String::from(host),
            hostname: String::new(),
            user: String::new(),
            identityfile: String::new(),
        }
    }
}

#[derive(PartialEq)]
enum Status {
    Before,
    Matching,
    After,
}

fn split_lines_on_host(lines: Vec<String>, host: &str) -> Result<SplitLines> {
    let mut status = Status::Before;
    let mut split_lines = SplitLines::new(host);
    for line in lines {
        let mut words = line.split_ascii_whitespace();
        match words.next() {
            Some("Host") => {
                if let Some(word) = words.next() {
                    if word == host && status == Status::Before {
                        status = Status::Matching;
                        split_lines.host = String::from(word);
                    };
                }
            }
            Some("HostName") => {
                if status == Status::Matching {
                    split_lines.hostname = String::from(words.next().unwrap_or(""));
                }
            }
            Some("User") => {
                if status == Status::Matching {
                    split_lines.user = String::from(words.next().unwrap_or(""));
                }
            }
            Some("IdentityFile") => {
                if status == Status::Matching {
                    split_lines.identityfile = String::from(words.next().unwrap_or(""));
                }
            }
            Some(_) => {}
            None => {
                if status == Status::Matching {
                    status = Status::After;
                }
            }
        }
        match status {
            Status::Before => split_lines.before.push(line),
            Status::Matching => {}
            Status::After => split_lines.after.push(line),
        };
    }
    match status {
        Status::Before => Err(anyhow!("Host {} not found in config file.", host)),
        _ => Ok(split_lines),
    }
}

fn rewrite_config_file(file_path: &PathBuf, split_lines: SplitLines) -> Result<()> {
    let mut file = OpenOptions::new().write(true).open(file_path.as_path())?;
    for line in split_lines.before {
        write!(file, "{}\n", line)?;
    }
    write!(file, "Host {}\n", split_lines.host)?;
    write!(file, "  HostName {}\n", split_lines.hostname)?;
    write!(file, "  User {}\n", split_lines.user)?;
    write!(file, "  IdentityFile {}\n", split_lines.identityfile)?;
    for line in split_lines.after {
        write!(file, "{}\n", line)?;
    }
    Ok(())
}
