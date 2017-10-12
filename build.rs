extern crate regex;

use std::fs::{self, File};

use std::io::{BufReader, Read, Write};

use regex::Regex;


// This build script will automatically find all public structs defined in
// this crate and create short test cases that simply attempt to `use`
// them. For example:
//     #[test]
//     fn use_blame_blame() {
//         use Blame;
//     }
//
// This should ensure that structs marked as `pub` in modules are
// properly re-exported in the top level module, and thus usable
// from another crate.


fn main() {
    // Filename to automatically generate test in.
    let test_filename = "src/test_use_pub_structs.rs";

    // Get list of public modules defined using `pub mod` in the main lib file
    // by reading whole file and using regular expressions.
    // Since those modules are publicly exported, their structs will be too,
    // but in their module namespace. Keep track of those modules so we can
    // later prefix the struct name with the module name.
    let f = File::open("src/lib.rs").expect("file src/lib.rs not found");
    let mut buf_reader = BufReader::new(f);
    let mut contents = String::new();
    buf_reader
        .read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    let pub_mods_re = Regex::new(r"pub mod (.*);").unwrap();
    let pub_mods: Vec<String> = pub_mods_re
        .captures_iter(&contents)
        .map(|cap| cap[1].into())
        .collect();

    // Initilize the content of the test file that we'll save later on.
    let mut test_file_content = String::with_capacity(8192); // Should be large enougth
    test_file_content.push_str("\n#![allow(unused_imports)]\n");

    // Get list of structs marked as public (through `pub struct`) and the module
    // they are defined in.

    // Compile the regular expression used to find public struct definitions.
    // Note that the regex expects the declaration to start at the beginning
    // of the line. This filters out structs defined in tests modules (and
    // hence indented).
    let pub_mods_and_structs_re = Regex::new(r"[^ ]pub struct ([^ <]*)").unwrap();

    for file_path in fs::read_dir("src/").unwrap() {
        let file_path = file_path.unwrap().path();
        // If rust file...
        if file_path.extension().unwrap().to_str() == Some("rs") {
            // Get the filename without extension and match only
            // those that are not named `lib`.
            match file_path.file_stem().unwrap().to_str() {
                Some(module) if module != "lib" => {
                    // Read file content
                    let f = File::open(&file_path)
                        .expect(&format!("could not open {}", file_path.display()));
                    let mut buf_reader = BufReader::new(f);
                    let mut contents = String::new();
                    buf_reader
                        .read_to_string(&mut contents)
                        .expect("something went wrong reading the file");

                    // Find the public structs in the file using regex.
                    let pub_structs: Vec<String> = pub_mods_and_structs_re
                        .captures_iter(&contents)
                        .map(|cap| cap[1].into())
                        .collect();

                    // For every public struct found, build a test case
                    // attempting to `use` the struct.
                    for pub_struct in pub_structs {
                        test_file_content.push_str("\n#[test]\nfn use_");
                        test_file_content.push_str(&module);
                        test_file_content.push_str("_");
                        test_file_content.push_str(&pub_struct.to_lowercase());
                        test_file_content.push_str("() {\n    use ");
                        // If module is publicly exported, use proper namespace to get the struct.
                        if pub_mods.contains(&module.into()) {
                            test_file_content.push_str(&module);
                            test_file_content.push_str("::");
                        }
                        test_file_content.push_str(&pub_struct);
                        test_file_content.push_str(";\n}\n");
                    }
                }
                _ => (),
            }
        };
    }

    // Write the test file
    let mut file = File::create(test_filename).expect("Could not open file");
    file.write_all(test_file_content.as_bytes())
        .expect("Could not write tests to file");
}
