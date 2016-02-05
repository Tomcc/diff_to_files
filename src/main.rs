use std::process::Command;

fn main() {
	let output = Command::new("git")
         .arg("log")
         .output()
         .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });

    println!("{:?}", output.stdout);
}
