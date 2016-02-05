#![feature(plugin)]
#![plugin(regex_macros)]

extern crate regex;
extern crate clap;
extern crate uuid;
use uuid::Uuid;

use std::io::BufWriter;
use std::io::prelude::*;
use std::process::Command;
use std::path::{Path, PathBuf};
use clap::{Arg, App};
use std::fs::File;

fn write_diff_file(lines: &Vec<String>, path: &Path, root: &Path) {
    let mut abspath = PathBuf::from(root);
    abspath.push(path);
    std::fs::create_dir_all(abspath.parent().unwrap()).unwrap();

    let file = File::create(abspath).unwrap();
    let mut writer = BufWriter::new(&file);
    for line in lines {
        write!(writer, "{}\n", line);
    }
}

fn main() {
    let matches = App::new("Make files out of an arbitrary diff")
                      .version("0.1")
                      .about("Still pretty incomplete")
                      .arg(Arg::with_name("from")
                               .help("commit ID, tag or branch name for the starting point")
                               .value_name("ID, tag or branch name")
                               .takes_value(true)
                               .required(true))
                      .arg(Arg::with_name("to")
                               .help("commit ID, tag or branch name for the base")
                               .value_name("ID, tag or branch name")
                               .takes_value(true)
                               .required(true))
                      .arg(Arg::with_name("config")
                               .help("Path to the JSON file containing the lint configuration. \
                                      Needed to perform linting")
                               .value_name("JSON config")
                               .long("config")
                               .short("c")
                               .takes_value(true))
                      .get_matches();

    let repo_path = PathBuf::from("C:/Users/tommaso/DEV/Minecraftpe");
    let mut out_path = std::env::temp_dir();
    out_path.push(Uuid::new_v4().to_simple_string());

    let output = Command::new("git")
                     .current_dir(repo_path.clone())
                     .arg("diff")
                     .arg(matches.value_of("from").unwrap())
                     .arg(matches.value_of("to").unwrap())
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));


    let logfile = String::from_utf8(output.stdout).unwrap();

    let mut diff_lines: Vec<String> = Vec::new();
    let mut file_path = PathBuf::new();
    let mut should_write_file = false;

    let file_start_re = regex!(r"(diff --git .* )(b/.*)$");
    let filter_re = regex!(r"\.cpp$|\.h$");
    let linefilter_re = regex!(r"\+\s");

    for line in logfile.split('\n') {
        if let Some(captures) = file_start_re.captures(line) {
            // this is a diff line// this is a diff line
            // write the old file if existing
            if diff_lines.len() > 0 && should_write_file {
                write_diff_file(&diff_lines, &file_path, &out_path);
            }

            // grab the filename from the line and make it absolute
            let filename = &captures.at(2).unwrap()[2..];
            should_write_file = filter_re.is_match(filename);
            if should_write_file {
                file_path = PathBuf::from(filename);
            }
            diff_lines.clear();

        } else if should_write_file && linefilter_re.is_match(line) {
            diff_lines.push(line[1..].to_owned())
        }
    }

    println!("Done! Changed sections exported to {:?}", out_path);

    // now run the linter if a config path is provided
    if let Some(config_path) = matches.value_of("config") {
        println!("Running the linter on the new files");
        let output = Command::new("acpplinter")
                 .current_dir(out_path)
                 .arg(config_path)
                 .output()
                 .unwrap_or_else(|e| panic!("Cannot run acpplinter: {}", e));

        println!("{}", String::from_utf8(output.stdout).unwrap());
    }
}
