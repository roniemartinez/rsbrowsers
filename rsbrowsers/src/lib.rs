use glob::{MatchOptions, Pattern};
use std::process::{Child, Command};
use std::vec::IntoIter;
#[cfg(target_os = "macos")]
use {plist::Value, std::path::Path};

#[cfg(target_os = "windows")]
use {
    pelite::FileMap,
    phf::{Map, phf_map},
    std::path::Path,
    winreg::RegKey,
    winreg::enums::HKEY_LOCAL_MACHINE,
};

#[cfg(target_os = "linux")]
use {
    freedesktop_desktop_entry::{DesktopEntry, Iter, default_paths},
    lazy_static::lazy_static,
    phf::{Map, phf_map},
    regex::Regex,
    std::fs,
};

#[cfg(target_os = "macos")]
const OSX_BROWSER_BUNDLE_LIST: &[(&str, &str, &str)] = &[
    // browser name, bundle ID, version string
    ("basilisk", "org.mozilla.basilisk", "CFBundleShortVersionString"),
    ("brave", "com.brave.Browser", "CFBundleVersion"),
    ("brave-beta", "com.brave.Browser.beta", "CFBundleVersion"),
    ("brave-dev", "com.brave.Browser.dev", "CFBundleVersion"),
    ("brave-nightly", "com.brave.Browser.nightly", "CFBundleVersion"),
    ("chrome", "com.google.Chrome", "CFBundleShortVersionString"),
    ("chrome-beta", "com.google.Chrome.beta", "CFBundleShortVersionString"),
    ("chrome-canary", "com.google.Chrome.canary", "CFBundleShortVersionString"),
    ("chrome-dev", "com.google.Chrome.dev", "CFBundleShortVersionString"),
    ("chrome-test", "com.google.chrome.for.testing", "CFBundleShortVersionString"),
    ("chromium", "org.chromium.Chromium", "CFBundleShortVersionString"),
    ("duckduckgo", "com.duckduckgo.macos.browser", "CFBundleShortVersionString"),
    ("epic", "com.hiddenreflex.Epic", "CFBundleShortVersionString"),
    ("firefox", "org.mozilla.firefox", "CFBundleShortVersionString"),
    ("firefox-developer", "org.mozilla.firefoxdeveloperedition", "CFBundleShortVersionString"),
    ("firefox-nightly", "org.mozilla.nightly", "CFBundleShortVersionString"),
    ("floorp", "org.mozilla.floorp", "CFBundleShortVersionString"),
    ("librewolf", "org.mozilla.librewolf", "CFBundleShortVersionString"),
    ("midori", "org.mozilla.midori", "CFBundleShortVersionString"),
    ("msedge", "com.microsoft.edgemac", "CFBundleShortVersionString"),
    ("msedge-beta", "com.microsoft.edgemac.Beta", "CFBundleShortVersionString"),
    ("msedge-dev", "com.microsoft.edgemac.Dev", "CFBundleShortVersionString"),
    ("msedge-canary", "com.microsoft.edgemac.Canary", "CFBundleShortVersionString"),
    ("opera", "com.operasoftware.Opera", "CFBundleVersion"),
    ("opera-beta", "com.operasoftware.OperaNext", "CFBundleVersion"),
    ("opera-developer", "com.operasoftware.OperaDeveloper", "CFBundleVersion"),
    ("opera-gx", "com.operasoftware.OperaGX", "CFBundleVersion"),
    ("opera-neon", "com.opera.Neon", "CFBundleShortVersionString"),
    ("pale-moon", "org.mozilla.pale moon", "CFBundleShortVersionString"),
    ("safari", "com.apple.Safari", "CFBundleShortVersionString"),
    ("safari-technology-preview", "com.apple.SafariTechnologyPreview", "CFBundleShortVersionString"),
    ("servo", "org.servo.Servo", "CFBundleShortVersionString"),
    ("vivaldi", "com.vivaldi.Vivaldi", "CFBundleShortVersionString"),
    ("waterfox", "net.waterfox.waterfox", "CFBundleShortVersionString"),
    ("yandex", "ru.yandex.desktop.yandex-browser", "CFBundleShortVersionString"),
    ("zen", "app.zen-browser.zen", "CFBundleShortVersionString"),
];

#[cfg(target_os = "windows")]
static WINDOWS_REGISTRY_BROWSER_NAMES: Map<&'static str, &'static str> = phf_map! {
    "Ablaze Floorp" => "floorp",
    "Basilisk" => "basilisk",
    "Brave" => "brave",
    "Brave Beta" => "brave-beta",
    "Brave Nightly" => "brave-nightly",
    "Chromium" => "chromium",
    "Firefox Developer Edition" => "firefox-developer",
    "Firefox Nightly" => "firefox-nightly",
    "Google Chrome" => "chrome",
    "Google Chrome Canary" => "chrome-canary",
    "Internet Explorer" => "msie",
    "LibreWolf" => "librewolf",
    "Microsoft Edge" => "msedge",
    "Microsoft Edge Beta" => "msedge-beta",
    "Microsoft Edge Dev" => "msedge-dev",
    "Microsoft Edge Canary" => "msedge-canary",
    "Mozilla Firefox" => "firefox",
    "Opera Stable" => "opera",
    "Opera beta" => "opera-beta",
    "Opera developer" => "opera-developer",
    "Pale Moon" => "pale-moon",
    "Waterfox" => "waterfox",
};

#[cfg(target_os = "linux")]
static LINUX_DESKTOP_ENTRY_NAME_LIST: Map<&'static str, &'static str> = phf_map! {
    // desktop entry name can be "brave-browser.desktop" or "brave_brave.desktop"
    "brave-browser" => "brave",
    "brave_brave" => "brave",
    "brave-browser-beta" => "brave-beta",
    "brave-browser-nightly" => "brave-nightly",
    "chromium" => "chromium",
    "chromium_chromium" => "chromium",
    "falkon_falkon" => "falkon",
    "firefox" => "firefox",
    "firefox_firefox" => "firefox",
    "google-chrome" => "chrome",
    "konqueror_konqueror" => "konqueror",
    "microsoft-edge" => "msedge",
    "opera_opera" => "opera",
    "opera-beta_opera-beta" => "opera-beta",
    "opera-developer_opera-developer" => "opera-developer",
    "vivaldi_vivaldi-stable" => "vivaldi",
};

#[cfg(target_os = "linux")]
lazy_static! {
    static ref VERSION_PATTERN: Regex = Regex::new(r"\b(\d+(\.\d+)+)\b").unwrap();
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    pub browser_type: String,
    pub path: String,
    pub display_name: String,
    pub version: String,
}

pub struct BrowserFinder {
    browser_type: String,
    version: String,
    exclude: String,
}

#[cfg(target_os = "macos")]
fn extract_info_from_plist(application_path: &str, browser_type: &str, version_string: &str) -> Browser {
    let base_path = Path::new(application_path);
    let path = base_path.join("Contents/Info.plist");
    let properties = Value::from_file(path).unwrap();

    let display_name = properties
        .as_dictionary()
        .and_then(|d| d.get("CFBundleDisplayName").or(d.get("CFBundleName")))
        .and_then(|e| e.as_string())
        .unwrap_or(browser_type);

    let executable_name =
        properties.as_dictionary().and_then(|d| d.get("CFBundleExecutable")).and_then(|e| e.as_string()).unwrap();

    let executable = match browser_type {
        "safari" => base_path.to_str().unwrap().to_owned(),
        _ => base_path.join("Contents/MacOS").join(executable_name).to_str().unwrap().to_owned(),
    };

    let version = properties.as_dictionary().and_then(|d| d.get(version_string)).and_then(|e| e.as_string()).unwrap();

    Browser {
        browser_type: browser_type.to_owned(),
        display_name: display_name.to_owned(),
        path: executable,
        version: version.to_owned(),
    }
}

#[cfg(target_os = "windows")]
fn get_version_info(path: &Path) -> String {
    // https://github.com/loot/loot-condition-interpreter/blob/2b95f26727f995b1b001b7ca9c9c233af9142c3d/src/function/version.rs#L139

    let mut version = "".to_string();

    if let Ok(file_map) = FileMap::open(path) {
        use pelite::pe64;

        version = match pe64::PeFile::from_bytes(file_map.as_ref()) {
            Ok(file) => {
                use pelite::pe64::Pe;

                let fixed_file_info = file.resources().unwrap().version_info().unwrap().fixed().unwrap();
                format!("{}", fixed_file_info.dwFileVersion)
            }
            Err(pelite::Error::PeMagic) => {
                use pelite::pe32::{Pe, PeFile};

                let fixed_file_info = PeFile::from_bytes(file_map.as_ref())
                    .unwrap()
                    .resources()
                    .unwrap()
                    .version_info()
                    .unwrap()
                    .fixed()
                    .unwrap();
                format!("{}", fixed_file_info.dwFileVersion)
            }
            Err(_) => version,
        }
    };

    version
}

impl BrowserFinder {
    pub fn new() -> Self {
        BrowserFinder { browser_type: String::from("*"), version: String::from("*"), exclude: String::from("") }
    }

    pub fn with_type(mut self, browser_type: String) -> Self {
        self.browser_type = browser_type;
        self
    }

    pub fn exclude_type(mut self, browser_type: String) -> Self {
        self.exclude = browser_type;
        self
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub fn all(&self) -> IntoIter<Browser> {
        let mut browsers = vec![];
        let browser_pattern = Pattern::new(self.browser_type.as_str()).unwrap();
        let version_pattern = Pattern::new(self.version.as_str()).unwrap();
        let exclude_pattern = Pattern::new(self.exclude.as_str()).unwrap();

        #[cfg(target_os = "macos")]
        for (browser_type, bundle_id, version_string) in OSX_BROWSER_BUNDLE_LIST.iter() {
            let result = Command::new("mdfind").arg(format!("kMDItemCFBundleIdentifier=='{bundle_id}'")).output();
            if let Ok(output) = result {
                browsers.extend(
                    String::from_utf8(output.stdout)
                        .unwrap()
                        .lines()
                        .map(String::from)
                        .map(|application| extract_info_from_plist(application.as_str(), browser_type, version_string))
                        .filter(|browser| {
                            Self::matches_patterns(browser, &browser_pattern, &version_pattern, &exclude_pattern)
                        })
                        .collect::<Vec<Browser>>(),
                )
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(smi) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"Software\Clients\StartMenuInternet") {
            for key in smi.enum_keys().map(|x| x.unwrap()) {
                if let Ok(browser) = smi.open_subkey(&key) {
                    let display_name: String = match browser.get_value("") {
                        Ok(display_name) => display_name,
                        Err(_) => key.to_string(),
                    };

                    if let Some(type_str) = WINDOWS_REGISTRY_BROWSER_NAMES.get(display_name.as_str()) {
                        if let Ok(command) = smi.open_subkey(format!(r"{key}\shell\open\command")) {
                            let mut path: String = match command.get_value("") {
                                Ok(command) => command,
                                Err(_) => continue,
                            };
                            path = match path.strip_prefix('"') {
                                Some(string) => string.to_string(),
                                None => path,
                            };
                            path = match path.strip_suffix('"') {
                                Some(string) => string.to_string(),
                                None => path,
                            };
                            let version = get_version_info(Path::new(path.as_str()));

                            let browser = Browser { browser_type: type_str.to_string(), display_name, path, version };

                            if Self::matches_patterns(&browser, &browser_pattern, &version_pattern, &exclude_pattern) {
                                browsers.push(browser);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        for path in Iter::new(default_paths()) {
            if let Ok(bytes) = fs::read_to_string(&path) {
                if let Ok(entry) = DesktopEntry::decode(&path, &bytes) {
                    let base_name = path.as_path().file_stem().unwrap().to_str().unwrap();
                    if LINUX_DESKTOP_ENTRY_NAME_LIST.contains_key(base_name) {
                        let browser_type = LINUX_DESKTOP_ENTRY_NAME_LIST[base_name].to_string();
                        let display_name = entry.name(None).unwrap().to_string();
                        let mut path = entry.exec().unwrap().to_string();
                        if path.to_lowercase().ends_with("%u") {
                            path.truncate(path.len() - 3);
                            path = path.trim().to_string();
                        }
                        let version = match Command::new("sh").arg("-c").arg(format!("{path} --version")).output() {
                            Ok(output) => {
                                let stdout = String::from_utf8(output.stdout).unwrap_or("".to_string());
                                match VERSION_PATTERN.captures(stdout.as_str()) {
                                    Some(capture) => capture.get(1).map_or("".to_string(), |m| m.as_str().to_string()),
                                    None => "".to_string(),
                                }
                            }
                            Err(_) => "".to_string(),
                        };

                        let browser = Browser { browser_type, display_name, path, version };

                        if Self::matches_patterns(&browser, &browser_pattern, &version_pattern, &exclude_pattern) {
                            browsers.push(browser);
                        }
                    }
                }
            }
        }

        browsers.into_iter()
    }

    fn matches_patterns(
        browser: &Browser,
        browser_pattern: &Pattern,
        version_pattern: &Pattern,
        exclude_pattern: &Pattern,
    ) -> bool {
        let case_insensitive = MatchOptions { case_sensitive: false, ..MatchOptions::new() };

        !exclude_pattern.matches_with(browser.browser_type.as_str(), case_insensitive)
            && version_pattern.matches_with(browser.version.as_str(), case_insensitive)
            && (browser_pattern.matches_with(browser.browser_type.as_str(), case_insensitive)
                | browser_pattern.matches_with(browser.display_name.as_str(), case_insensitive))
    }

    pub fn launch(&self, args: &[String]) -> (Child, Browser) {
        let browser = self.all().next().unwrap();

        match browser.browser_type.as_str() {
            #[cfg(target_os = "macos")]
            "safari" => {
                let mut arguments = vec![
                    "--wait-apps".to_owned(),
                    "--new".to_owned(),
                    "--fresh".to_owned(),
                    "-a".to_owned(),
                    browser.path.to_owned(),
                ];
                arguments.extend_from_slice(args);

                return (Command::new("open").args(arguments).spawn().unwrap(), browser);
            }
            _ => {
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                return (Command::new(&browser.path).args(args).spawn().unwrap(), browser);
                #[cfg(target_os = "linux")]
                return (
                    Command::new("sh").arg("-c").arg(format!("{} {}", browser.path, args.join(" "))).spawn().unwrap(),
                    browser,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::BrowserFinder;

    #[test]
    fn test_all() {
        let browsers = BrowserFinder::new().all().map(|browser| browser.browser_type).collect::<Vec<String>>();

        assert!(browsers.contains(&"chrome".to_string()));
        assert!(browsers.contains(&"firefox".to_string()));
        #[cfg(target_os = "macos")]
        assert!(browsers.contains(&"safari".to_string()));
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        assert!(browsers.contains(&"msedge".to_string()));
        #[cfg(target_os = "windows")]
        assert!(browsers.contains(&"internet-explorer".to_string()));
    }
}
