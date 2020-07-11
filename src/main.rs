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
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("CONFIG FILE")
                .help("Custom path for SSH config file")
                .takes_value(true),
        )
        .get_matches();

    update_config(
        matches.value_of("host").unwrap(),
        matches.value_of("hostname").unwrap(),
        matches.value_of("config"),
    )
}

fn update_config(host: &str, new_hostname: &str, custom_path: Option<&str>) -> Result<()> {
    let config_path = match custom_path {
        Some(path) => PathBuf::from(path),
        None => hardcoded_config_location().context("Failed to find path for config file.")?,
    };
    let config_lines = read_config_file(&config_path)
        .context(format!("Problem reading config file: {:?}", config_path))?;
    let mut split_lines = split_lines_on_host(config_lines, &host)?;
    if split_lines.hostname != new_hostname {
        let old_hostname = split_lines.hostname;
        split_lines.hostname = String::from(new_hostname);
        rewrite_config_file(&config_path, &split_lines)
            .context("Error while writing new config file.")?;
        println!(
            "Modified '{}' hostname from '{}' to '{}'.",
            &host, &old_hostname, &new_hostname
        );
    } else {
        println!(
            "No change: '{}' hostname is already '{}'.",
            &host, &new_hostname
        );
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
        None => Err(anyhow!("Failed to determine home directory.")),
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
        match (words.next(), &status) {
            (Some("Host"), Status::Before) => {
                if words.next().unwrap_or("") == host {
                    status = Status::Matching;
                    split_lines.host = String::from(host);
                }
            }
            (Some("HostName"), Status::Matching) => {
                split_lines.hostname = String::from(words.next().unwrap_or(""));
            }
            (Some("User"), Status::Matching) => {
                split_lines.user = String::from(words.next().unwrap_or(""));
            }
            (Some("IdentityFile"), Status::Matching) => {
                split_lines.identityfile = String::from(words.next().unwrap_or(""));
            }
            (None, Status::Matching) => {
                status = Status::After;
            }
            _ => {}
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

fn rewrite_config_file(file_path: &PathBuf, split_lines: &SplitLines) -> Result<()> {
    let mut file = OpenOptions::new().write(true).open(file_path.as_path())?;
    for line in &split_lines.before {
        write!(file, "{}\n", line)?;
    }
    write!(file, "Host {}\n", split_lines.host)?;
    write!(file, "  HostName {}\n", split_lines.hostname)?;
    write!(file, "  User {}\n", split_lines.user)?;
    write!(file, "  IdentityFile {}\n", split_lines.identityfile)?;
    for line in &split_lines.after {
        write!(file, "{}\n", line)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_lines() -> Vec<String> {
        let mut lines = Vec::<String>::new();
        lines.push(String::from("Host firsthost"));
        lines.push(String::from("  HostName 1.2.3.4"));
        lines.push(String::from("  User noah"));
        lines.push(String::from("  IdentityFile ~/.ssh/id_rsa"));
        lines.push(String::from(""));
        lines.push(String::from("Host secondhost"));
        lines.push(String::from("  HostName 99.99.99.99"));
        lines.push(String::from("  User alice"));
        lines.push(String::from("  IdentityFile ~/.ssh/id_rsa"));
        lines.push(String::from(""));
        lines.push(String::from("Host thirdhost"));
        lines.push(String::from("  HostName 127.0.0.1"));
        lines.push(String::from("  User bob"));
        lines.push(String::from("  IdentityFile ~/.ssh/id_rsa"));
        lines.push(String::from(""));
        lines.push(String::from("Host fourthhost"));
        lines.push(String::from("  HostName 127.9.9.1"));
        lines.push(String::from("  User carly"));
        lines.push(String::from("  IdentityFile ~/.ssh/id_rsa"));
        lines.push(String::from(""));
        lines
    }

    #[test]
    fn before_lines() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.before.len(),
            5
        ))
    }

    #[test]
    fn after_lines() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.after.len(),
            11
        ))
    }

    #[test]
    fn host() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.host,
            "secondhost"
        ))
    }

    #[test]
    fn hostname() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.hostname,
            "99.99.99.99"
        ))
    }

    #[test]
    fn user() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.user,
            "alice"
        ))
    }

    #[test]
    fn identityfile() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "secondhost")?.identityfile,
            "~/.ssh/id_rsa"
        ))
    }

    #[test]
    fn fourthhost() -> Result<()> {
        Ok(assert_eq!(
            split_lines_on_host(gen_lines(), "fourthhost")?.hostname,
            "127.9.9.1"
        ))
    }
}
