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
use regex::Regex;
use clap::{Arg, App};
use std::fs::File;

static BLAME_RE: Regex = regex!("^.*?\\(.*?\\)");

fn blame(file_path: &Path) -> String {
    let output = Command::new("git")
                     .arg("blame")
                     .arg(file_path)
                     .output()
                     .unwrap_or_else(|e| panic!("Cannot run acpplinter: {}", e));

    String::from_utf8(output.stdout).unwrap()
}

#[derive(Debug)]
struct Line {
    id: usize,
    text: String,
}

impl Line {
    fn from_line_number(text: &str, id: usize) -> Self {
        Line {
            text: text.to_owned(),
            id: id,
        }
    }
}

fn write_diff_file(diffs: &Vec<Line>, path: &Path, root: &Path) {
    println!("Writing {:?}", path);

    let mut abspath = PathBuf::from(root);
    abspath.push(path);
    std::fs::create_dir_all(abspath.parent().unwrap()).unwrap();

    let blame_path = abspath.to_string_lossy().into_owned() + ".blame";

    let file = File::create(abspath).unwrap();
    let blame_file = File::create(blame_path).unwrap();

    let mut writer = BufWriter::new(&file);
    let mut blame_writer = BufWriter::new(&blame_file);

    let mut blame_line_idx = 0;
    let mut diff_line_idx = 0;
    for blame_line in blame(path).split('\n') {
        blame_line_idx += 1;

        if blame_line.len() == 0 {
            continue;
        }

        //check if this line is in the diff
        let diff_line = &diffs[diff_line_idx];
        if blame_line_idx == diff_line.id {
            write!(writer, "{}\n", &diff_line.text);
            write!(blame_writer, "{}\n", BLAME_RE.captures(blame_line).unwrap().at(0).unwrap());
            diff_line_idx += 1;

            //no more diffs available
            if diff_line_idx == diffs.len() {
                return;
            }
        }
    }
}

fn main() {
    let matches = App::new("Make files out of an arbitrary diff")
                      .version("0.1")
                      .about("Still pretty incomplete")
                      .arg(Arg::with_name("id_range")
                               .help("A git object range, in any of the forms allowed by git. \
                                      For example, commit...commit, commit..branch, \
                                      branch...tag and so on.")
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
                     .arg("-U0")
                     .arg(matches.value_of("id_range").unwrap())
                     .output()
                     .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    let logfile = String::from_utf8(output.stdout).unwrap();

    if logfile.len() == 0 {
        println!("Git error:");
        println!("{:?}", String::from_utf8(output.stderr).unwrap());
        process::exit(1);
    }

    let mut diff_lines: Vec<Line> = Vec::new();
    let mut file_path = PathBuf::new();

    let file_start_re = regex!(r"(diff --git .* )(b/.*)$");
    let linefilter_re = regex!(r"^\+[^\+]");
    let line_info_re = regex!(r"^@@.*?\+([0-9]+)");
    let plus = "+".to_string();
    let mut current_line = 0;

    for line in logfile.split('\n') {
        if let Some(captures) = line_info_re.captures(line) {
            current_line = captures.at(1).unwrap().parse::<usize>().unwrap();
        } else if let Some(captures) = file_start_re.captures(line) {
            // this is a diff line// this is a diff line
            // write the old file if existing
            if diff_lines.len() > 0 {
                write_diff_file(&diff_lines, &file_path, &out_path);
            }
    
            // grab the filename from the line and make it absolute
            file_path = PathBuf::from(&captures.at(2).unwrap()[2..]);
            diff_lines.clear();
        } else if linefilter_re.is_match(line) || line == plus {
            diff_lines.push(Line::from_line_number(&line[1..], current_line));
            current_line += 1;
        }
    }

    println!("Done! Changed sections exported to {}",
             out_path.to_string_lossy());

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
