use std::sync::{Arc, Mutex};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum WebSearchError {
    #[error("search query is empty")]
    EmptyQuery,

    #[error("web search engine not found: {0}")]
    EngineNotFound(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("insecure or unsupported URI scheme: {0}")]
    InsecureScheme(String),

    #[error("failed to launch web browser: {0}")]
    LaunchFailed(String),

    #[error("failed to parse web engines XML: {0}")]
    ParseXml(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebEngine {
    pub id: String,
    pub name: String,
    pub template_url: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct WebSearchCatalog {
    engines: Vec<WebEngine>,
    default_engine_id: String,
}

impl Default for WebSearchCatalog {
    fn default() -> Self {
        Self::default_catalog()
    }
}

impl WebSearchCatalog {
    pub fn default_catalog() -> Self {
        let engines = vec![
            WebEngine {
                id: "google".into(),
                name: "Google".into(),
                template_url: "https://www.google.com/search?q=[$key]".into(),
                category: "general".into(),
            },
            WebEngine {
                id: "bing".into(),
                name: "Bing".into(),
                template_url: "https://www.bing.com/search?q=[$key]".into(),
                category: "general".into(),
            },
            WebEngine {
                id: "wikipedia".into(),
                name: "Wikipedia".into(),
                template_url: "https://en.wikipedia.org/wiki/[$key]".into(),
                category: "encyclopedia".into(),
            },
            WebEngine {
                id: "oxford".into(),
                name: "Oxford Learners".into(),
                template_url:
                    "https://www.oxfordlearnersdictionaries.com/definition/english/[$key]".into(),
                category: "dictionary".into(),
            },
            WebEngine {
                id: "cambridge".into(),
                name: "Cambridge".into(),
                template_url: "https://dictionary.cambridge.org/search/english/direct/?q=[$key]"
                    .into(),
                category: "dictionary".into(),
            },
            WebEngine {
                id: "vdict".into(),
                name: "VDict".into(),
                template_url: "https://vdict.com/[$key],1,0,0.html".into(),
                category: "dictionary".into(),
            },
        ];

        Self {
            engines,
            default_engine_id: "google".into(),
        }
    }

    pub fn engines(&self) -> &[WebEngine] {
        &self.engines
    }

    pub fn default_engine_id(&self) -> &str {
        &self.default_engine_id
    }

    pub fn set_default_engine_id(&mut self, id: impl Into<String>) {
        self.default_engine_id = id.into();
    }

    pub fn get_engine(&self, id: &str) -> Option<&WebEngine> {
        self.engines.iter().find(|e| e.id.eq_ignore_ascii_case(id))
    }

    pub fn add_engine(&mut self, engine: WebEngine) {
        if let Some(pos) = self
            .engines
            .iter()
            .position(|e| e.id.eq_ignore_ascii_case(&engine.id))
        {
            self.engines[pos] = engine;
        } else {
            self.engines.push(engine);
        }
    }

    pub fn build_url(&self, engine_id: Option<&str>, query: &str) -> Result<Url, WebSearchError> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(WebSearchError::EmptyQuery);
        }

        let target_id = engine_id.unwrap_or(&self.default_engine_id);
        let engine = self
            .get_engine(target_id)
            .ok_or_else(|| WebSearchError::EngineNotFound(target_id.to_string()))?;

        let encoded_query =
            url::form_urlencoded::byte_serialize(trimmed_query.as_bytes()).collect::<String>();
        let rendered_url = engine
            .template_url
            .replace("[$key]", &encoded_query)
            .replace("[$langcode]", "en");

        let parsed =
            Url::parse(&rendered_url).map_err(|e| WebSearchError::InvalidUrl(e.to_string()))?;

        match parsed.scheme() {
            "https" | "http" => Ok(parsed),
            other => Err(WebSearchError::InsecureScheme(other.to_string())),
        }
    }

    pub fn from_xml(xml: &str) -> Result<Self, WebSearchError> {
        let mut engines = Vec::new();

        // Simple XML engine attribute parser for webengine.xml compatibility
        for line in xml.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("<engine ") && trimmed.ends_with("/>") {
                let id = extract_xml_attr(trimmed, "id").unwrap_or_default();
                let name = extract_xml_attr(trimmed, "name").unwrap_or_default();
                let url = extract_xml_attr(trimmed, "url").unwrap_or_default();
                if !id.is_empty() && !url.is_empty() {
                    engines.push(WebEngine {
                        id,
                        name: if name.is_empty() {
                            "Web Engine".into()
                        } else {
                            name
                        },
                        template_url: url,
                        category: "general".into(),
                    });
                }
            }
        }

        if engines.is_empty() {
            return Err(WebSearchError::ParseXml(
                "no valid <engine> nodes found".into(),
            ));
        }

        let default_id = engines[0].id.clone();
        Ok(Self {
            engines,
            default_engine_id: default_id,
        })
    }
}

fn extract_xml_attr(line: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("&amp;", "&"))
}

pub trait BrowserLauncher: Send + Sync {
    fn open_url(&self, url: &Url) -> Result<(), WebSearchError>;
}

#[derive(Default, Clone)]
pub struct DefaultBrowserLauncher;

impl BrowserLauncher for DefaultBrowserLauncher {
    fn open_url(&self, url: &Url) -> Result<(), WebSearchError> {
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows::Win32::UI::Shell::ShellExecuteW;
            use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            use windows::core::PCWSTR;

            let wide_url: Vec<u16> = OsStr::new(url.as_str())
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let wide_op: Vec<u16> = OsStr::new("open")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                let instance = ShellExecuteW(
                    None,
                    PCWSTR(wide_op.as_ptr()),
                    PCWSTR(wide_url.as_ptr()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
                if (instance.0 as usize) <= 32 {
                    return Err(WebSearchError::LaunchFailed(format!(
                        "ShellExecuteW returned code {}",
                        instance.0 as usize
                    )));
                }
            }
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let cmd = if cfg!(target_os = "macos") {
                "open"
            } else {
                "xdg-open"
            };
            std::process::Command::new(cmd)
                .arg(url.as_str())
                .spawn()
                .map_err(|e| WebSearchError::LaunchFailed(e.to_string()))?;
            Ok(())
        }
    }
}

#[derive(Default, Clone)]
pub struct MockBrowserLauncher {
    history: Arc<Mutex<Vec<String>>>,
}

impl MockBrowserLauncher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recorded_urls(&self) -> Vec<String> {
        self.history.lock().unwrap().clone()
    }
}

impl BrowserLauncher for MockBrowserLauncher {
    fn open_url(&self, url: &Url) -> Result<(), WebSearchError> {
        self.history.lock().unwrap().push(url.to_string());
        Ok(())
    }
}
