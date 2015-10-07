extern crate ctest;

use std::env;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var_os("DEP_GIT2_ROOT").unwrap());

    let mut cfg = ctest::TestGenerator::new();
    cfg.header("git2.h")
       .header("git2/sys/transport.h")
       .header("git2/cred_helpers.h")
       .include(root.join("include"))
       .type_name(|s, _| s.to_string());
    cfg.field_name(|_, f| {
        match f {
            "kind" => "type".to_string(),
            _ => f.to_string(),
        }
    });
    cfg.skip_signededness(|s| {
        match s {
            s if s.ends_with("_cb") => true,
            s if s.ends_with("_callback") => true,
            "git_push_transfer_progress" |
            "git_push_negotiation" |
            "git_packbuilder_progress" => true,
            _ => false,
        }
    });
    cfg.skip_type(|t| t == "__enum_ty");
    cfg.generate("../libgit2-sys/lib.rs", "all.rs");
}
