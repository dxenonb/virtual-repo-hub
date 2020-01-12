mod repo_gen;

use crate::repo_gen::{
    execute_yaml,
    AssertionError,
};

use std::path::PathBuf;
use std::fs;
use std::env;

struct Test {
    path: PathBuf,
    name: String,
    error: Option<(usize, AssertionError)>,
}

#[test]
fn repo_status_suite() {
    let working_dir = env::current_dir()
        .unwrap();

    let entries = fs::read_dir("tests/repo_status")
        .expect("failed to read directory");

    let mut results = Vec::new();

    for entry in entries {
        env::set_current_dir(&working_dir)
            .expect("failed to update working directory");

        let entry = entry.expect("failed to read entry");
        let path = entry.path();
        let name = path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if name.ends_with(".yaml") {
            let r = if let Err(error) = execute_yaml(&path) {
                Test {
                    path,
                    name,
                    error: Some(error),
                }
            } else {
                Test {
                    path,
                    name,
                    error: None,
                }
            };

            results.push(r);
        }
    }

    let mut successes = 0;
    let mut failures = 0;

    println!("\n\tRepo Status test suite complete\n");
    for t in results {
        if let Some((index, error)) = &t.error {
            println!("\t\tERROR: {}, command {}", &t.name, index);
            println!("\t\t@ {:?}", &t.path);
            println!("\t\tExpected:\n\t\t\t{:?}", &error.expected);
            println!("\t\tGot:\n\t\t\t{:?}\n", &error.actual);
            failures += 1;
        } else {
            println!("\t\tSUCCESS: {}\n", &t.name);
            successes += 1;
        }
    }

    println!("\t== {} succeeded, {} failed ==\n", successes, failures);
    if failures != 0 {
        panic!("repo status error");
    }
}
