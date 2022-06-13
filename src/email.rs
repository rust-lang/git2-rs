use std::ffi::CString;
use std::{mem, ptr};

use crate::util::Binding;
use crate::{raw, Buf, Commit, DiffFindOptions, DiffOptions, Error, IntoCString};
use crate::{Diff, Oid, Signature};

/// A structure to represent patch in mbox format for sending via email
pub struct Email {
    buf: Buf,
}

/// Options for controlling the formatting of the generated e-mail.
pub struct EmailCreateOptions {
    diff_options: DiffOptions,
    diff_find_options: DiffFindOptions,
    subject_prefix: Option<CString>,
    raw: raw::git_email_create_options,
}

impl Default for EmailCreateOptions {
    fn default() -> Self {
        // Defaults options created in corresponding to `GIT_EMAIL_CREATE_OPTIONS_INIT`
        let default_options = raw::git_email_create_options {
            version: raw::GIT_EMAIL_CREATE_OPTIONS_VERSION,
            flags: raw::GIT_EMAIL_CREATE_DEFAULT as u32,
            diff_opts: unsafe { mem::zeroed() },
            diff_find_opts: unsafe { mem::zeroed() },
            subject_prefix: ptr::null(),
            start_number: 1,
            reroll_number: 0,
        };
        let mut diff_options = DiffOptions::new();
        diff_options.show_binary(true).context_lines(3);
        Self {
            diff_options,
            diff_find_options: DiffFindOptions::new(),
            subject_prefix: None,
            raw: default_options,
        }
    }
}

impl EmailCreateOptions {
    /// Creates a new set of email create options
    ///
    /// By default, options include rename detection and binary
    /// diffs to match `git format-patch`.
    pub fn new() -> Self {
        Self::default()
    }

    fn flag(&mut self, opt: raw::git_email_create_flags_t, val: bool) -> &mut Self {
        let opt = opt as u32;
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Flag indicating whether patch numbers are included in the subject prefix.
    pub fn omit_numbers(&mut self, omit: bool) -> &mut Self {
        self.flag(raw::GIT_EMAIL_CREATE_OMIT_NUMBERS, omit)
    }

    /// Flag indicating whether numbers included in the subject prefix even when
    /// the patch is for a single commit (1/1).
    pub fn always_number(&mut self, always: bool) -> &mut Self {
        self.flag(raw::GIT_EMAIL_CREATE_ALWAYS_NUMBER, always)
    }

    /// Flag indicating whether rename or similarity detection are ignored.
    pub fn ignore_renames(&mut self, ignore: bool) -> &mut Self {
        self.flag(raw::GIT_EMAIL_CREATE_NO_RENAMES, ignore)
    }

    /// Get mutable access to `DiffOptions` that are used for creating diffs.
    pub fn diff_options(&mut self) -> &mut DiffOptions {
        &mut self.diff_options
    }

    /// Get mutable access to `DiffFindOptions` that are used for finding
    /// similarities within diffs.
    pub fn diff_find_options(&mut self) -> &mut DiffFindOptions {
        &mut self.diff_find_options
    }

    /// Set the subject prefix
    ///
    /// The default value for this is "PATCH". If set to an empty string ("")
    /// then only the patch numbers will be shown in the prefix.
    /// If the subject_prefix is empty and patch numbers are not being shown,
    /// the prefix will be omitted entirely.
    pub fn subject_prefix<T: IntoCString>(&mut self, t: T) -> &mut Self {
        self.subject_prefix = Some(t.into_c_string().unwrap());
        self
    }

    /// Set the starting patch number; this cannot be 0.
    ///
    /// The default value for this is 1.
    pub fn start_number(&mut self, number: usize) -> &mut Self {
        self.raw.start_number = number;
        self
    }

    /// Set the "re-roll" number.
    ///
    /// The default value for this is 0 (no re-roll).
    pub fn reroll_number(&mut self, number: usize) -> &mut Self {
        self.raw.reroll_number = number;
        self
    }

    /// Acquire a pointer to the underlying raw options.
    ///
    /// This function is unsafe as the pointer is only valid so long as this
    /// structure is not moved, modified, or used elsewhere.
    unsafe fn raw(&mut self) -> *const raw::git_email_create_options {
        self.raw.subject_prefix = self
            .subject_prefix
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());
        self.raw.diff_opts = ptr::read(self.diff_options.raw());
        self.raw.diff_find_opts = ptr::read(self.diff_find_options.raw());
        &self.raw as *const _
    }
}

impl Email {
    /// Returns a byte slice with stored e-mail patch in. `Email` could be
    /// created by one of the `from_*` functions.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    /// Create a diff for a commit in mbox format for sending via email.
    pub fn from_diff<T: IntoCString>(
        diff: &Diff<'_>,
        patch_idx: usize,
        patch_count: usize,
        commit_id: &Oid,
        summary: T,
        body: T,
        author: &Signature<'_>,
        opts: &mut EmailCreateOptions,
    ) -> Result<Self, Error> {
        let buf = Buf::new();
        let summary = summary.into_c_string()?;
        let body = body.into_c_string()?;
        unsafe {
            try_call!(raw::git_email_create_from_diff(
                buf.raw(),
                Binding::raw(diff),
                patch_idx,
                patch_count,
                Binding::raw(commit_id),
                summary.as_ptr(),
                body.as_ptr(),
                Binding::raw(author),
                opts.raw()
            ));
            Ok(Self { buf })
        }
    }

    /// Create a diff for a commit in mbox format for sending via email.
    /// The commit must not be a merge commit.
    pub fn from_commit(commit: &Commit<'_>, opts: &mut EmailCreateOptions) -> Result<Self, Error> {
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_email_create_from_commit(
                buf.raw(),
                commit.raw(),
                opts.raw()
            ));
            Ok(Self { buf })
        }
    }
}
