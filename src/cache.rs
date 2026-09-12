use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use home::home_dir;

pub fn get_lopy_dir() -> PathBuf {
    let home = home_dir().expect("no home dir");
    Path::new(&home).join(".lopy")
}

pub fn get_cache_dir() -> PathBuf {
    get_lopy_dir().join("cache")
}

pub fn ensure_dirs() {
    fs::create_dir_all(get_cache_dir()).expect("Failed to create cache dir");
}

pub fn hash_string(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}