#![feature(plugin)]
#![plugin(regex_macros)]

extern crate regex;
extern crate clap;
extern crate uuid;
use uuid::Uuid;

use std::io::BufWriter;
use std::io::prelude::*;
use std::process::Command;
use std::process;
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
                      .arg(Arg::with_name("id_range")
                               .help("A git object range, in any of the forms allowed by git. For example, commit...commit, commit..branch, branch...tag and so on.")
                               .value_name("Git ID range")
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

    let mut out_path = std::env::temp_dir();
    out_path.push(Uuid::new_v4().to_simple_string());

    let output = Command::new("git")
                     .arg("diff")
                     .arg(matches.value_of("id_range").unwrap())
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    let logfile = String::from_utf8(output.stdout).unwrap();

    if logfile.len() == 0 {
        println!("Git error:");
        println!("{:?}", String::from_utf8(output.stderr).unwrap());
        process::exit(1);
    }

    let mut diff_lines: Vec<String> = Vec::new();
    let mut file_path = PathBuf::new();

    let file_start_re = regex!(r"(diff --git .* )(b/.*)$");
    let linefilter_re = regex!(r"^\+\s");

    for line in logfile.split('\n') {
        if let Some(captures) = file_start_re.captures(line) {
            // this is a diff line// this is a diff line
            // write the old file if existing
            if diff_lines.len() > 0 {
                write_diff_file(&diff_lines, &file_path, &out_path);
            }

            // grab the filename from the line and make it absolute
            file_path = PathBuf::from(&captures.at(2).unwrap()[2..]);
            diff_lines.clear();

        } else if linefilter_re.is_match(line) {
            diff_lines.push(line[1..].to_owned())
        }
    }

    println!("Done! Changed sections exported to {:?}", out_path);

    // now run the linter if a config path is provided
    if let Some(config_path) = matches.value_of("config") {
        println!("Running the linter on the new files");
        let output = Command::new("acpplinter")
                         .arg(config_path)
                         .arg(out_path)
                         .output()
                         .unwrap_or_else(|e| panic!("Cannot run acpplinter: {}", e));

        println!("{}", String::from_utf8(output.stdout).unwrap());
        println!("{}", String::from_utf8(output.stderr).unwrap());
    }
}
