use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use std::process::Command;
use home::home_dir;
use semver::Version;
use serde::{Deserialize, Serialize};
use toml;

const REPO_FILE: &str = "repo.toml";

#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TomlProject {
    pub package: TomlPackage,
    pub dependencies: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TomlPackage {
    pub name: String,
    pub version: String,
}

pub fn get_lopy_dir() -> PathBuf {
    let home = home_dir().expect("no home dir");
    Path::new(&home).join(".lopy")
}

pub fn get_packages_dir() -> PathBuf {
    get_lopy_dir().join("packages")
}

pub fn get_repo_file() -> PathBuf {
    get_lopy_dir().join(REPO_FILE)
}

pub fn ensure_dirs() {
    fs::create_dir_all(get_packages_dir()).expect("Failed to create packages dir");
}

pub fn read_repo_map() -> std::collections::HashMap<String, String> {
    let repo_path = get_repo_file();
    if !repo_path.exists() {
        return std::collections::HashMap::new();
    }
    let content = fs::read_to_string(repo_path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_else(|_| std::collections::HashMap::new())
}

pub fn add_package(name: &str, version: &str, url: Option<&str>) {
    ensure_dirs();
    let packages_dir = get_packages_dir();
    let target_dir = packages_dir.join(format!("{}@{}", name, version));

    if target_dir.exists() {
        println!("⚠️  Пакет {}@{} уже установлен", name, version);
        return;
    }

    let repo_map = read_repo_map();
    let repo_url = if let Some(url) = url {
        url.to_string()
    } else if let Some(repo_url) = repo_map.get(name) {
        repo_url.clone()
    } else {
        panic!("⚠️  Пакет {} не найден в репозитории. Укажите URL.", name);
    };

    let final_url = if version == "latest" {
        format!("{}/archive/refs/heads/main.zip", repo_url)
    } else {
        format!("{}/archive/refs/tags/v{}.zip", repo_url, version)
    };

    println!("⬇️  Установка {}@{} из {}", name, version, final_url);
    download_and_extract_package(&final_url, &target_dir, name);
}

fn download_and_extract_package(url: &str, target_dir: &Path, name: &str) {
    let response = reqwest::blocking::get(url).expect("Failed to download package");
    let bytes = response.bytes().expect("Failed to read bytes");

    let temp_file = target_dir.with_extension("tmp");
    fs::write(&temp_file, bytes).expect("Failed to write temp file");

    let extract_temp = target_dir.parent().unwrap().join(format!("{}_extract", name));
    if extract_temp.exists() {
        fs::remove_dir_all(&extract_temp).unwrap();
    }
    fs::create_dir_all(&extract_temp).unwrap();

    let ext = url.split('.').last().unwrap_or("zip");
    if ext == "zip" {
        let file = fs::File::open(&temp_file).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        archive.extract(&extract_temp).expect("Failed to extract zip");
    } else if ext == "tar.gz" || ext == "tgz" {
        let file = fs::File::open(&temp_file).unwrap();
        let tar = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&extract_temp).expect("Failed to extract tar.gz");
    } else {
        panic!("⚠️  Неизвестный формат архива: {}", ext);
    }

    // Если target_dir существует - удаляем
    if target_dir.exists() {
        fs::remove_dir_all(target_dir).unwrap();
    }
    fs::create_dir_all(target_dir).unwrap();

    // Функция рекурсивного копирования
    fn copy_recursive(src: &Path, dst: &Path) {
        if src.is_dir() {
            fs::create_dir_all(dst).unwrap();
            for entry in fs::read_dir(src).unwrap() {
                let entry = entry.unwrap();
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                copy_recursive(&src_path, &dst_path);
            }
        } else {
            fs::copy(src, dst).unwrap();
        }
    }

    // Находим корневую папку в архиве (GitHub добавляет папку с именем репозитория)
    let entries: Vec<_> = fs::read_dir(&extract_temp).unwrap().collect();
    if entries.len() == 1 {
        let entry = entries[0].as_ref().unwrap();
        let path = entry.path();
        if path.is_dir() {
            // Копируем содержимое папки
            for sub_entry in fs::read_dir(&path).unwrap() {
                let sub_entry = sub_entry.unwrap();
                let src_path = sub_entry.path();
                let dest_path = target_dir.join(sub_entry.file_name());
                copy_recursive(&src_path, &dest_path);
            }
        } else {
            // Если внутри файл, копируем его
            copy_recursive(&path, &target_dir.join(entry.file_name()));
        }
    } else {
        // Если внутри несколько папок, просто копируем всё
        for entry in entries {
            let entry = entry.unwrap();
            let src_path = entry.path();
            let dest_path = target_dir.join(entry.file_name());
            copy_recursive(&src_path, &dest_path);
        }
    }

    // Очистка
    fs::remove_file(temp_file).unwrap();
    fs::remove_dir_all(extract_temp).unwrap();
    println!("✅ {} установлен", target_dir.file_name().unwrap().to_string_lossy());
}

pub fn compile_package(name: &str, version: &str) -> Result<(PathBuf, Vec<String>), String> {
    let pkg_dir = get_packages_dir().join(format!("{}@{}", name, version));
    let lib_dir = pkg_dir.join("lib");

    // Определяем скрипт для платформы
    let script_name = if cfg!(target_os = "linux") {
        format!("{}_linux.lb", name)
    } else if cfg!(target_os = "windows") {
        format!("{}_windows.bat", name)
    } else if cfg!(target_os = "macos") {
        format!("{}_macos.lb", name)
    } else {
        return Err("Неизвестная платформа".to_string());
    };

    let script_path = lib_dir.join(&script_name);

    // Если скрипта нет - пропускаем
    if !script_path.exists() {
        eprintln!("⚠️  Скрипт {} не найден для пакета {}", script_name, name);
        return Ok((PathBuf::new(), Vec::new()));
    }

    // ДЕЛАЕМ СКРИПТ ИСПОЛНЯЕМЫМ!
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
    }

    // Запускаем скрипт из lib/ (где лежат .c файлы)
    eprintln!("🔧 Компиляция пакета {}...", name);
    let (shell, args) = if cfg!(windows) {
        ("cmd", vec!["/C", script_path.to_str().unwrap()])
    } else {
        ("sh", vec![script_path.to_str().unwrap()])
    };

    let output = Command::new(shell)
        .args(&args)
        .current_dir(&lib_dir)
        .output()
        .map_err(|e| format!("Не удалось запустить скрипт {}: {}", script_path.display(), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("📤 STDOUT: {}", stdout);
        eprintln!("📤 STDERR: {}", stderr);
        return Err(format!("Ошибка сборки пакета {}: {}", name, stderr));
    }

    // Считываем флаги из stdout (echo)
    let flags_str = String::from_utf8_lossy(&output.stdout);
    let flags: Vec<String> = flags_str
        .lines()
        .flat_map(|line| line.split_whitespace())
        .map(|s| s.to_string())
        .collect();

    // Ищем скомпилированный .o в lib/
    let compiled_path = lib_dir.join(format!("{}.o", name));

    if !compiled_path.exists() {
        return Err(format!("❌ Скомпилированный файл {}.o не найден в lib/", name));
    }

    eprintln!("✅ Пакет {} скомпилирован", name);
    Ok((compiled_path, flags))
}

pub fn get_package_path(name: &str, version: &str) -> PathBuf {
    if version == "local" {
        get_packages_dir().join(format!("{}@local", name))
    } else {
        get_packages_dir().join(format!("{}@{}", name, version))
    }
}

pub fn resolve_package(name: &str, version: &str) -> PathBuf {
    if version == "latest" {
        // 1. Проверяем удалённые версии
        match get_remote_versions(name) {
            Ok(remote_versions) => {
                if let Some(remote_max) = remote_versions.first() {
                    let pkg_path = get_package_path(name, remote_max);
                    if !pkg_path.exists() {
                        add_package(name, remote_max, None);
                    }
                    create_latest_symlink(name, remote_max);
                    return pkg_path;
                }
            }
            Err(e) => {
                println!("⚠️ Не удалось получить удалённые версии: {}", e);
            }
        }

        // 2. Fallback - локальные версии
        let mut local_versions = Vec::new();
        if let Ok(entries) = fs::read_dir(get_packages_dir()) {
            for entry in entries.flatten() {
                let name_str = entry.file_name().to_string_lossy().to_string();
                if name_str.starts_with(&format!("{}@", name)) {
                    let ver = name_str.trim_start_matches(&format!("{}@", name));
                    if ver != "local" && ver != "latest" {
                        if let Ok(v) = Version::parse(ver) {
                            local_versions.push(v);
                        }
                    }
                }
            }
        }

        if let Some(local_max) = local_versions.iter().max() {
            let ver_str = local_max.to_string();
            create_latest_symlink(name, &ver_str);
            return get_package_path(name, &ver_str);
        }

        panic!("❌ Пакет {} не найден", name);
    }

    // 2. Проверяем конкретную версию
    let pkg_path = get_package_path(name, version);
    if pkg_path.exists() {
        return pkg_path;
    }
    
    add_package(name, version, None);
    get_package_path(name, version)
}

pub fn get_remote_versions(name: &str) -> Result<Vec<String>, String> {
    return Err("Отключено для теста".parse().unwrap());
    let repo_map = read_repo_map();
    let repo_url = repo_map.get(name).ok_or_else(|| format!("Пакет {} не найден в репозитории", name))?;

    let api_url = repo_url
        .replace("github.com", "api.github.com/repos")
        .replace("https://", "https://");

    let url = format!("{}/tags", api_url);

    // Создаём клиент с User-Agent
    let client = reqwest::blocking::Client::builder()
        .user_agent("lopy-pkg/1.0.0")
        .build()
        .map_err(|e| format!("Ошибка создания клиента: {}", e))?;

    // Добавляем заголовки
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .map_err(|e| format!("Не удалось получить версии: {}", e))?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            // Пробуем без API - через страницу релизов
            let releases_url = repo_url.replace("github.com", "api.github.com/repos") + "/releases";
            let response = client
                .get(&releases_url)
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .map_err(|e| format!("Не удалось получить релизы: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("GitHub API недоступен: {}", response.status()));
            }

            let releases: Vec<serde_json::Value> = response.json()
                .map_err(|e| format!("Ошибка парсинга релизов: {}", e))?;

            let mut versions = Vec::new();
            for release in releases {
                if let Some(tag) = release["tag_name"].as_str() {
                    let ver = tag.trim_start_matches('v');
                    if let Ok(v) = Version::parse(ver) {
                        versions.push(v);
                    }
                }
            }

            versions.sort_by(|a, b| b.cmp(a));
            return Ok(versions.iter().map(|v| v.to_string()).collect());
        }
        return Err(format!("API вернул ошибку: {}", response.status()));
    }

    let tags: Vec<serde_json::Value> = response.json()
        .map_err(|e| format!("Ошибка парсинга: {}", e))?;

    let mut versions = Vec::new();
    for tag in tags {
        if let Some(name) = tag["name"].as_str() {
            let ver = name.trim_start_matches('v');
            if let Ok(v) = Version::parse(ver) {
                versions.push(v);
            }
        }
    }

    versions.sort_by(|a, b| b.cmp(a));
    Ok(versions.iter().map(|v| v.to_string()).collect())
}

// Создание симлинка для latest
fn create_latest_symlink(name: &str, version: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let latest_link = get_packages_dir().join(format!("{}@latest", name));
        let target = get_packages_dir().join(format!("{}@{}", name, version));

        // Удаляем старый симлинк
        if latest_link.exists() {
            let _ = std::fs::remove_file(&latest_link);
        }

        // Создаём новый
        if let Err(e) = symlink(&target, &latest_link) {
            eprintln!("⚠️ Не удалось создать симлинк для {}@latest: {}", name, e);
        }
    }
}

pub fn remove_package(name: &str) {
    let packages_dir = get_packages_dir();
    let mut found = false;

    for entry in fs::read_dir(packages_dir).unwrap() {
        let entry = entry.unwrap();
        let name_str = entry.file_name().to_string_lossy().to_string();
        if name_str.starts_with(&format!("{}@", name)) {
            fs::remove_dir_all(entry.path()).expect("Failed to remove package");
            found = true;
            println!("✅ {} удалён", name_str);
        }
    }

    if !found {
        println!("⚠️  Пакет {} не найден", name);
    }
}

pub fn list_packages() {
    let packages_dir = get_packages_dir();
    if !packages_dir.exists() {
        println!("⚠️  Нет установленных пакетов");
        return;
    }

    let entries = fs::read_dir(packages_dir).unwrap();
    let mut packages: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    packages.sort();

    if packages.is_empty() {
        println!("⚠️  Нет установленных пакетов");
    } else {
        println!("📦 Установленные пакеты:");
        for pkg in packages {
            println!("  - {}", pkg);
        }
    }
}

pub fn read_project_toml() -> Option<TomlProject> {
    let toml_path = Path::new("lopy.toml");
    if !toml_path.exists() {
        return None;
    }
    let content = fs::read_to_string(toml_path).expect("Failed to read lopy.toml");
    match toml::from_str(&content) {
        Ok(project) => {
            Some(project)
        },
        Err(e) => {
            println!("❌ Ошибка парсинга: {}", e);
            None
        }
    }
}

pub fn parse_c_signatures(c_file: &Path) -> HashMap<String, String> {
    let content = match fs::read_to_string(c_file) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };

    let mut signatures = HashMap::new();

    // НОВАЯ РЕГУЛЯРКА — ловит char*, void*, const char*
    let re = regex::Regex::new(
        r"(?m)^\s*(?:extern\s+)?([a-zA-Z_][a-zA-Z0-9_]*\s*[*\s]*)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*(?:;|\{?)"
    ).unwrap();

    for cap in re.captures_iter(&content) {
        let ret_type_raw = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("void");
        let func_name = cap.get(2).unwrap().as_str().to_string();
        let params = cap.get(3).unwrap().as_str();

        // Пропускаем статические функции и макросы
        if ret_type_raw.starts_with("static") || ret_type_raw.starts_with("inline") || ret_type_raw.starts_with("typedef") {
            continue;
        }

        if func_name.starts_with('_') || func_name == "main" {
            continue;
        }

        let llvm_ret = c_to_llvm_type(ret_type_raw);
        signatures.insert(func_name, llvm_ret);
    }

    signatures
}

fn c_to_llvm_type(c_type: &str) -> String {
    let trimmed = c_type.trim();
    if trimmed.is_empty() { return "void".to_string(); }

    // Убираем const, volatile, unsigned
    let clean = trimmed.replace("const", "").replace("volatile", "").replace("unsigned", "").trim().to_string();

    // Проверяем на указатель
    if clean.contains('*') {
        return "i8*".to_string();
    }

    match clean.as_str() {
        "void" => "void".to_string(),
        "char" => "i8".to_string(),
        "short" => "i16".to_string(),
        "int" => "i32".to_string(),
        "long" => "i64".to_string(),
        "long long" => "i64".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        "__int128" => "i128".to_string(),
        _ => {
            // Если неизвестный тип — пробуем i128
            "i128".to_string()
        }
    }
}