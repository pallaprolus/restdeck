use tui_textarea::TextArea;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Sidebar,
    RequestUrl,
    RequestTabContent,
    Response,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RequestTab {
    Headers,
    Params,
    Body,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidebarSelection {
    Request(usize),
    Environment(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: String,
    pub params: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Environment {
    pub name: String,
    pub variables: String, // Newline-separated "key=value"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub collections: Vec<ApiRequest>,
    pub environments: Vec<Environment>,
    pub active_env_index: Option<usize>,
}

pub struct App<'a> {
    pub collections: Vec<ApiRequest>,
    pub environments: Vec<Environment>,
    pub active_env_index: Option<usize>,
    pub sidebar_index: usize, // Flat index across requests and environments
    pub focus: Focus,
    pub request_tab: RequestTab,
    pub method_index: usize,
    
    // Text inputs
    pub url_textarea: TextArea<'a>,
    pub headers_textarea: TextArea<'a>,
    pub params_textarea: TextArea<'a>,
    pub body_textarea: TextArea<'a>,
    pub env_textarea: TextArea<'a>,
    
    // HTTP Response state
    pub response_content: String,
    pub response_status: Option<String>,
    pub response_time: Option<String>,
    pub response_size: Option<String>,
    pub is_loading: bool,
    pub response_scroll: u16,
}

pub const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH"];

fn default_environments() -> Vec<Environment> {
    vec![
        Environment {
            name: "Dev".to_string(),
            variables: "baseUrl=https://httpbin.org\napiKey=dev_key_xyz".to_string(),
        },
        Environment {
            name: "Prod".to_string(),
            variables: "baseUrl=https://httpbin.org\napiKey=prod_key_abc".to_string(),
        },
    ]
}

fn default_requests() -> Vec<ApiRequest> {
    vec![
        ApiRequest {
            name: "Get User Info".to_string(),
            method: "GET".to_string(),
            url: "{{baseUrl}}/json".to_string(),
            headers: "Accept: application/json\nUser-Agent: RestDeck-TUI\nAuthorization: Bearer {{apiKey}}".to_string(),
            params: "limit=10\npage=1".to_string(),
            body: "".to_string(),
        },
        ApiRequest {
            name: "Create User".to_string(),
            method: "POST".to_string(),
            url: "{{baseUrl}}/post".to_string(),
            headers: "Content-Type: application/json".to_string(),
            params: "".to_string(),
            body: "{\n  \"name\": \"Eleanor Vance\",\n  \"role\": \"admin\"\n}".to_string(),
        },
        ApiRequest {
            name: "Update User".to_string(),
            method: "PUT".to_string(),
            url: "{{baseUrl}}/put".to_string(),
            headers: "Content-Type: application/json".to_string(),
            params: "".to_string(),
            body: "{\n  \"role\": \"super-admin\"\n}".to_string(),
        },
        ApiRequest {
            name: "Delete User".to_string(),
            method: "DELETE".to_string(),
            url: "{{baseUrl}}/delete".to_string(),
            headers: "".to_string(),
            params: "".to_string(),
            body: "".to_string(),
        },
        ApiRequest {
            name: "Delayed Response (Test Async)".to_string(),
            method: "GET".to_string(),
            url: "{{baseUrl}}/delay/3".to_string(),
            headers: "".to_string(),
            params: "".to_string(),
            body: "".to_string(),
        },
    ]
}

fn get_config_path() -> std::path::PathBuf {
    if cfg!(test) {
        let thread_name = std::thread::current().name().unwrap_or("test").to_string();
        let sanitized_name = thread_name.replace("::", "_");
        return std::path::PathBuf::from(format!("restdeck_test_{}.json", sanitized_name));
    }
    
    let local_path = std::path::PathBuf::from("restdeck.json");
    if local_path.exists() {
        return local_path;
    }
    
    if let Ok(home) = std::env::var("HOME") {
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("restdeck");
        path.push("collections.json");
        return path;
    }

    local_path
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let (collections, environments, active_env_index) = Self::load_config().unwrap_or_else(|| {
            (default_requests(), default_environments(), Some(0))
        });

        let mut app = Self {
            collections,
            environments,
            active_env_index,
            sidebar_index: 0,
            focus: Focus::Sidebar,
            request_tab: RequestTab::Headers,
            method_index: 0,
            url_textarea: TextArea::default(),
            headers_textarea: TextArea::default(),
            params_textarea: TextArea::default(),
            body_textarea: TextArea::default(),
            env_textarea: TextArea::default(),
            response_content: "Welcome to RestDeck!\n\nUse Tab / Shift-Tab to switch panels.\nUse Ctrl-E to trigger HTTP request.\nUse Ctrl-H/Ctrl-P/Ctrl-B to switch request tabs.\nUse Up/Down to navigate sidebar requests.".to_string(),
            response_status: None,
            response_time: None,
            response_size: None,
            is_loading: false,
            response_scroll: 0,
        };

        app.load_sidebar_selection(0);
        app
    }

    pub fn load_config() -> Option<(Vec<ApiRequest>, Vec<Environment>, Option<usize>)> {
        let path = get_config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&data) {
                    return Some((config.collections, config.environments, config.active_env_index));
                }
                if let Ok(collections) = serde_json::from_str::<Vec<ApiRequest>>(&data) {
                    return Some((collections, default_environments(), Some(0)));
                }
            }
        }
        None
    }

    pub fn save_config(&self) {
        let path = get_config_path();
        
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let config = AppConfig {
            collections: self.collections.clone(),
            environments: self.environments.clone(),
            active_env_index: self.active_env_index,
        };

        if let Ok(json_str) = serde_json::to_string_pretty(&config) {
            let _ = std::fs::write(&path, json_str);
        }
    }

    pub fn get_sidebar_selection(&self) -> SidebarSelection {
        if self.sidebar_index < self.collections.len() {
            SidebarSelection::Request(self.sidebar_index)
        } else {
            SidebarSelection::Environment(self.sidebar_index - self.collections.len())
        }
    }

    pub fn total_sidebar_items(&self) -> usize {
        self.collections.len() + self.environments.len()
    }

    pub fn load_sidebar_selection(&mut self, index: usize) {
        self.sidebar_index = index;
        match self.get_sidebar_selection() {
            SidebarSelection::Request(idx) => {
                let req = &self.collections[idx];
                
                // Load URL
                self.url_textarea = TextArea::new(vec![req.url.clone()]);
                self.url_textarea.set_cursor_line_style(ratatui::style::Style::default());
                self.url_textarea.set_cursor_style(ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::REVERSED));

                // Load Method
                if let Some(pos) = HTTP_METHODS.iter().position(|&m| m == req.method) {
                    self.method_index = pos;
                } else {
                    self.method_index = 0;
                }

                // Load Headers
                let headers_lines: Vec<String> = req.headers.lines().map(|s| s.to_string()).collect();
                self.headers_textarea = TextArea::new(headers_lines);
                self.headers_textarea.set_cursor_line_style(ratatui::style::Style::default());

                // Load Params
                let params_lines: Vec<String> = req.params.lines().map(|s| s.to_string()).collect();
                self.params_textarea = TextArea::new(params_lines);
                self.params_textarea.set_cursor_line_style(ratatui::style::Style::default());

                // Load Body
                let body_lines: Vec<String> = req.body.lines().map(|s| s.to_string()).collect();
                self.body_textarea = TextArea::new(body_lines);
                self.body_textarea.set_cursor_line_style(ratatui::style::Style::default());
            }
            SidebarSelection::Environment(idx) => {
                let env = &self.environments[idx];
                let env_lines: Vec<String> = env.variables.lines().map(|s| s.to_string()).collect();
                self.env_textarea = TextArea::new(env_lines);
                self.env_textarea.set_cursor_line_style(ratatui::style::Style::default());
            }
        }
    }

    pub fn save_current_request(&mut self) {
        match self.get_sidebar_selection() {
            SidebarSelection::Request(idx) => {
                if idx >= self.collections.len() {
                    return;
                }
                let url = self.url_textarea.lines()[0].trim().to_string();
                let method = HTTP_METHODS[self.method_index].to_string();
                let headers = self.headers_textarea.lines().join("\n");
                let params = self.params_textarea.lines().join("\n");
                let body = self.body_textarea.lines().join("\n");

                let req = &mut self.collections[idx];
                req.url = url;
                req.method = method;
                req.headers = headers;
                req.params = params;
                req.body = body;
            }
            SidebarSelection::Environment(idx) => {
                if idx >= self.environments.len() {
                    return;
                }
                let variables = self.env_textarea.lines().join("\n");
                self.environments[idx].variables = variables;
            }
        }

        self.save_config();
    }

    pub fn get_interpolated_request(&self) -> ApiRequest {
        let selection = self.get_sidebar_selection();
        let idx = match selection {
            SidebarSelection::Request(i) => i,
            _ => 0,
        };

        if idx >= self.collections.len() {
            return default_requests()[0].clone();
        }

        let req = &self.collections[idx];
        let mut interpolated = req.clone();
        
        let mut vars = Vec::new();
        if let Some(env_idx) = self.active_env_index {
            if env_idx < self.environments.len() {
                let env = &self.environments[env_idx];
                for line in env.variables.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(pos) = line.find('=') {
                        let key = line[..pos].trim().to_string();
                        let val = line[pos + 1..].trim().to_string();
                        vars.push((key, val));
                    }
                }
            }
        }
        
        fn replace_all(mut text: String, vars: &[(String, String)]) -> String {
            for (key, val) in vars {
                let placeholder = format!("{{{{{}}}}}", key);
                text = text.replace(&placeholder, val);
            }
            text
        }
        
        interpolated.url = replace_all(interpolated.url, &vars);
        interpolated.headers = replace_all(interpolated.headers, &vars);
        interpolated.params = replace_all(interpolated.params, &vars);
        interpolated.body = replace_all(interpolated.body, &vars);
        
        interpolated
    }

    pub fn cycle_method(&mut self) {
        self.method_index = (self.method_index + 1) % HTTP_METHODS.len();
        self.save_current_request();
    }

    pub fn cycle_focus(&mut self, forward: bool) {
        self.save_current_request();
        self.focus = match (self.focus, forward) {
            (Focus::Sidebar, true) => {
                match self.get_sidebar_selection() {
                    SidebarSelection::Request(_) => Focus::RequestUrl,
                    SidebarSelection::Environment(_) => Focus::RequestTabContent,
                }
            }
            (Focus::RequestUrl, true) => Focus::RequestTabContent,
            (Focus::RequestTabContent, true) => Focus::Response,
            (Focus::Response, true) => Focus::Sidebar,

            (Focus::Sidebar, false) => Focus::Response,
            (Focus::RequestUrl, false) => Focus::Sidebar,
            (Focus::RequestTabContent, false) => {
                match self.get_sidebar_selection() {
                    SidebarSelection::Request(_) => Focus::RequestUrl,
                    SidebarSelection::Environment(_) => Focus::Sidebar,
                }
            }
            (Focus::Response, false) => Focus::RequestTabContent,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cleanup_test_file() {
        let path = get_config_path();
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_new_app() {
        cleanup_test_file();
        let app = App::new();
        assert_eq!(app.sidebar_index, 0);
        assert_eq!(app.focus, Focus::Sidebar);
        assert_eq!(app.collections.len(), 5);
        cleanup_test_file();
    }

    #[test]
    fn test_cycle_focus() {
        cleanup_test_file();
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Sidebar);
        
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::RequestUrl);

        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::RequestTabContent);

        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Response);
        cleanup_test_file();
    }

    #[test]
    fn test_interpolation() {
        let mut app = App::new();
        app.environments[0].variables = "baseUrl=https://api.restdeck.com\napiKey=foo-token".to_string();
        app.active_env_index = Some(0);
        
        app.collections[0].url = "{{baseUrl}}/v1/users".to_string();
        app.collections[0].headers = "Authorization: Bearer {{apiKey}}".to_string();
        
        let interpolated = app.get_interpolated_request();
        assert_eq!(interpolated.url, "https://api.restdeck.com/v1/users");
        assert_eq!(interpolated.headers, "Authorization: Bearer foo-token");
    }

    #[test]
    fn test_cycle_method() {
        cleanup_test_file();
        let mut app = App::new();
        assert_eq!(app.method_index, 0); // GET
        
        app.cycle_method();
        assert_eq!(app.method_index, 1); // POST
        assert_eq!(app.collections[0].method, "POST");
        cleanup_test_file();
    }

    #[test]
    fn test_save_load_config() {
        cleanup_test_file();
        
        let mut app = App::new();
        app.collections[0].url = "https://example.com/config-test".to_string();
        app.environments[0].name = "Staging".to_string();
        app.save_config();
        
        let (collections, envs, _) = App::load_config().unwrap();
        assert_eq!(collections[0].url, "https://example.com/config-test");
        assert_eq!(envs[0].name, "Staging");
        
        cleanup_test_file();
    }
}
