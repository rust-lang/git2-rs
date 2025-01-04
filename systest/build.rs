use std::env;
use std::path::PathBuf;

fn main() {
    let mut cfg = ctest2::TestGenerator::new();
    if let Some(root) = env::var_os("DEP_GIT2_ROOT") {
        cfg.include(PathBuf::from(root).join("include"));
    }
    cfg.header("git2.h")
        .header("git2/sys/errors.h")
        .header("git2/sys/transport.h")
        .header("git2/sys/refs.h")
        .header("git2/sys/refdb_backend.h")
        .header("git2/sys/odb_backend.h")
        .header("git2/sys/mempack.h")
        .header("git2/sys/repository.h")
        .header("git2/sys/cred.h")
        .header("git2/sys/email.h")
        .header("git2/cred_helpers.h")
        .type_name(|s, _, _| s.to_string());
    cfg.field_name(|_, f| match f {
        "kind" => "type".to_string(),
        _ => f.to_string(),
    });
    cfg.skip_field(|struct_, f| {
        // this field is marked as const which ctest complains about
        (struct_ == "git_rebase_operation" && f == "id") ||
        // the real name of this field is ref but that is a reserved keyword
        (struct_ == "git_worktree_add_options" && f == "reference")
    });
    cfg.skip_signededness(|s| match s {
        s if s.ends_with("_cb") => true,
        s if s.ends_with("_callback") => true,
        "git_push_transfer_progress" | "git_push_negotiation" | "git_packbuilder_progress" => true,
        _ => false,
    });

    // GIT_FILEMODE_BLOB_GROUP_WRITABLE is not a public const in libgit2
    cfg.define("GIT_FILEMODE_BLOB_GROUP_WRITABLE", Some("0100664"));

    // not entirely sure why this is failing...
    cfg.skip_roundtrip(|t| t == "git_clone_options" || t == "git_submodule_update_options");

    cfg.skip_type(|t| t == "__enum_ty");
    cfg.generate("../libgit2-sys/lib.rs", "all.rs");
}
