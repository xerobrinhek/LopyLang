use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use reqwest;

const LIBS_REPO: &str = "xerobrinhek/lopy-libs";
const VERSION_FILE: &str = "version.txt";

pub fn get_libs_dir() -> PathBuf {
    let home = env::var("HOME").expect("HOME not set");
    Path::new(&home).join(".lopy/libs")
}

pub fn get_installed_version() -> Option<String> {
    let version_path = get_libs_dir().join(VERSION_FILE);
    if version_path.exists() {
        Some(fs::read_to_string(version_path).unwrap_or_default().trim().to_string())
    } else {
        None
    }
}

pub fn get_latest_version() -> Result<String, reqwest::Error> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", LIBS_REPO);
    let client = reqwest::blocking::Client::builder()
        .user_agent("lopy-compiler")
        .build()?;
    let response: serde_json::Value = client.get(&url).send()?.json()?;
    let tag = response["tag_name"].as_str().unwrap_or("v0.0.0");
    Ok(tag.trim_start_matches('v').to_string())
}

pub fn download_and_extract_libs(version: &str) {
    let libs_dir = get_libs_dir();
    if libs_dir.exists() {
        fs::remove_dir_all(&libs_dir).expect("Failed to remove old libs");
    }
    fs::create_dir_all(&libs_dir).expect("Failed to create libs dir");

    let url = format!(
        "https://github.com/{}/releases/download/v{}/libs.zip",
        LIBS_REPO, version
    );

    println!("⬇️  Скачивание библиотек v{}...", version);
    let response = reqwest::blocking::get(&url).expect("Failed to download libs");
    let bytes = response.bytes().expect("Failed to read bytes");

    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).expect("Failed to open ZIP");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = libs_dir.join(file.name());
        if file.is_dir() {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }
    }

    let version_path = libs_dir.join(VERSION_FILE);
    fs::write(version_path, version).expect("Failed to write version file");
}

pub fn ensure_libs() {
    let latest = match get_latest_version() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("⚠️  Не удалось проверить версию библиотек. Использую локальную.");
            return;
        }
    };

    let installed = get_installed_version();

    if installed.as_deref() != Some(&latest) {
        println!("📦 Обновление библиотек: {} -> {}", installed.unwrap_or_else(|| "нет".to_string()), latest);
        download_and_extract_libs(&latest);
    }
}

pub fn get_lib_path() -> PathBuf {
    get_libs_dir()
}

pub fn get_std_lib_path() -> PathBuf {
    get_libs_dir().join("llstd")
}

pub fn get_gui_lib_path() -> PathBuf {
    get_libs_dir().join("llgui")
}