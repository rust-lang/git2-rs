fn main() {
    git2::opts::enable_caching(true);
    git2::opts::strict_object_creation(true);
    git2::opts::strict_hash_verification(true);
}
