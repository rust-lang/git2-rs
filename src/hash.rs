use oid::Oid;
use std::mem::*;
use raw;
use util::Binding;
/// A function that hashes the data passed to it, using the git default hash function.
///
/// Use this for hashing data that needs to integrate with git, as this will change with git when it changes hash from SHA-1.
pub fn hash(data : &[u8]) -> Result<Oid, ()> {
    unsafe{
        // Allocate a git_oid for the hash
        let mut oid : raw::git_oid = uninitialized();
        // Hash the data, and place the result in oid
        let error = raw::git_hash_buf(&mut oid, data.as_ptr() as _, data.len());
        // Create the high level oid object from the raw pointer
        let oid : Oid = Oid::from_raw(&oid);
        // Check the return code
        if error != 0 { Err(()) } else { Ok(oid) }
    }
}
