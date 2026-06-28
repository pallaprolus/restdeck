mod app;
mod http;
mod ui;

use app::{App, Focus, RequestTab, SidebarMode, SidebarSelection, HTTP_METHODS};
use http::HttpResponseEvent;
use tui_textarea::TextArea;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Create channel for async HTTP events
    let (tx, mut rx) = mpsc::channel::<HttpResponseEvent>(10);

    // Event loop
    loop {
        // Draw TUI
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Read channel events (non-blocking)
        while let Ok(event) = rx.try_recv() {
            match event {
                HttpResponseEvent::Start => {
                    app.is_loading = true;
                    app.response_content = String::new();
                    app.response_status = None;
                    app.response_time = None;
                    app.response_size = None;
                    app.response_scroll = 0;
                }
                HttpResponseEvent::Success {
                    body,
                    status,
                    time_ms,
                    size_bytes,
                    raw_url,
                    raw_method,
                    raw_headers,
                    raw_params,
                    raw_body,
                } => {
                    app.is_loading = false;
                    app.response_content = body.clone();
                    
                    let status_str = status.clone();
                    let time_str = format!("{}ms", time_ms);
                    let size_str = format!("{:.2} KB", size_bytes as f64 / 1024.0);
                    
                    app.response_status = Some(status_str.clone());
                    app.response_time = Some(time_str.clone());
                    app.response_size = Some(size_str.clone());
                    
                    // Add run to history
                    app.add_history_item(
                        raw_url,
                        raw_method,
                        raw_headers,
                        raw_params,
                        raw_body,
                        Some(status_str),
                        Some(time_str),
                        Some(size_str),
                        body,
                    );
                }
                HttpResponseEvent::Error {
                    err,
                    raw_url,
                    raw_method,
                    raw_headers,
                    raw_params,
                    raw_body,
                } => {
                    app.is_loading = false;
                    app.response_content = err.clone();
                    app.response_status = Some("Error".to_string());
                    
                    app.add_history_item(
                        raw_url,
                        raw_method,
                        raw_headers,
                        raw_params,
                        raw_body,
                        Some("Error".to_string()),
                        None,
                        None,
                        err,
                    );
                }
            }
        }

        // Wait for crossterm event
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                // Global keyboard actions
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                    break;
                }
                if app.focus == Focus::Sidebar && key.code == KeyCode::Esc {
                    break;
                }

                // Global Toggle for Sidebar Mode (Collections <-> History)
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('y') {
                    app.sidebar_mode = match app.sidebar_mode {
                        SidebarMode::Collections => SidebarMode::History,
                        SidebarMode::History => SidebarMode::Collections,
                    };
                    app.focus = Focus::Sidebar;
                    
                    // If switching to history, load the currently selected history item details
                    if app.sidebar_mode == SidebarMode::History && !app.history.is_empty() {
                        let idx = app.history_index;
                        if idx < app.history.len() {
                            let item = &app.history[idx];
                            app.url_textarea = TextArea::new(vec![item.url.clone()]);
                            app.headers_textarea = TextArea::new(item.headers.lines().map(|s| s.to_string()).collect());
                            app.params_textarea = TextArea::new(item.params.lines().map(|s| s.to_string()).collect());
                            app.body_textarea = TextArea::new(item.body.lines().map(|s| s.to_string()).collect());
                            if let Some(pos) = HTTP_METHODS.iter().position(|&m| m == item.method) {
                                app.method_index = pos;
                            }
                        }
                    } else if app.sidebar_mode == SidebarMode::Collections {
                        app.load_sidebar_selection(app.sidebar_index);
                    }
                    continue;
                }

                // Intercept navigation before textareas get it
                let is_tab = key.code == KeyCode::Tab;
                let is_backtab = key.code == KeyCode::BackTab;

                if is_tab {
                    app.cycle_focus(true);
                    continue;
                }
                if is_backtab {
                    app.cycle_focus(false);
                    continue;
                }

                // Trigger Request
                if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('e') {
                    app.save_current_request();
                    let raw_req = match app.get_sidebar_selection() {
                        SidebarSelection::Request(idx) => app.collections[idx].clone(),
                        _ => app.collections[0].clone(),
                    };
                    
                    let req = app.get_interpolated_request();
                    http::run_request(
                        req.url.clone(),
                        req.method.clone(),
                        req.headers.clone(),
                        req.params.clone(),
                        req.body.clone(),
                        raw_req.url,
                        raw_req.method,
                        raw_req.headers,
                        raw_req.params,
                        raw_req.body,
                        tx.clone(),
                    );
                    app.focus = Focus::Response;
                    continue;
                }

                // Switch tabs (only if request is selected in Collections mode)
                if key.modifiers == KeyModifiers::CONTROL 
                    && app.sidebar_mode == SidebarMode::Collections
                    && matches!(app.get_sidebar_selection(), SidebarSelection::Request(_)) 
                {
                    match key.code {
                        KeyCode::Char('h') => {
                            app.request_tab = RequestTab::Headers;
                            continue;
                        }
                        KeyCode::Char('p') => {
                            app.request_tab = RequestTab::Params;
                            continue;
                        }
                        KeyCode::Char('b') => {
                            app.request_tab = RequestTab::Body;
                            continue;
                        }
                        KeyCode::Char('m') => {
                            app.cycle_method();
                            continue;
                        }
                        _ => {}
                    }
                }

                // Focus specific navigation
                match app.focus {
                    Focus::Sidebar => match app.sidebar_mode {
                        SidebarMode::Collections => match key.code {
                            KeyCode::Down | KeyCode::Char('j') => {
                                let total = app.total_sidebar_items();
                                let next = (app.sidebar_index + 1) % total;
                                app.load_sidebar_selection(next);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                let total = app.total_sidebar_items();
                                let prev = if app.sidebar_index == 0 {
                                    total - 1
                                } else {
                                    app.sidebar_index - 1
                                };
                                app.load_sidebar_selection(prev);
                            }
                            KeyCode::Enter => {
                                match app.get_sidebar_selection() {
                                    SidebarSelection::Request(_) => {
                                        app.focus = Focus::RequestUrl;
                                    }
                                    SidebarSelection::Environment(idx) => {
                                        app.active_env_index = Some(idx);
                                        app.save_config();
                                    }
                                }
                            }
                            _ => {}
                        },
                        SidebarMode::History => match key.code {
                            KeyCode::Down | KeyCode::Char('j') => {
                                if !app.history.is_empty() {
                                    app.history_index = (app.history_index + 1) % app.history.len();
                                    
                                    // Update inputs to show the details of the highlighted run
                                    let idx = app.history_index;
                                    let item = &app.history[idx];
                                    app.url_textarea = TextArea::new(vec![item.url.clone()]);
                                    app.headers_textarea = TextArea::new(item.headers.lines().map(|s| s.to_string()).collect());
                                    app.params_textarea = TextArea::new(item.params.lines().map(|s| s.to_string()).collect());
                                    app.body_textarea = TextArea::new(item.body.lines().map(|s| s.to_string()).collect());
                                    if let Some(pos) = HTTP_METHODS.iter().position(|&m| m == item.method) {
                                        app.method_index = pos;
                                    }
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if !app.history.is_empty() {
                                    app.history_index = if app.history_index == 0 {
                                        app.history.len() - 1
                                    } else {
                                        app.history_index - 1
                                    };
                                    
                                    // Update inputs to show the details of the highlighted run
                                    let idx = app.history_index;
                                    let item = &app.history[idx];
                                    app.url_textarea = TextArea::new(vec![item.url.clone()]);
                                    app.headers_textarea = TextArea::new(item.headers.lines().map(|s| s.to_string()).collect());
                                    app.params_textarea = TextArea::new(item.params.lines().map(|s| s.to_string()).collect());
                                    app.body_textarea = TextArea::new(item.body.lines().map(|s| s.to_string()).collect());
                                    if let Some(pos) = HTTP_METHODS.iter().position(|&m| m == item.method) {
                                        app.method_index = pos;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if !app.history.is_empty() {
                                    app.restore_history_request();
                                }
                            }
                            _ => {}
                        }
                    },
                    Focus::RequestUrl => {
                        app.url_textarea.input(key);
                        app.save_current_request();
                    }
                    Focus::RequestTabContent => {
                        match app.sidebar_mode {
                            SidebarMode::Collections => {
                                match app.get_sidebar_selection() {
                                    SidebarSelection::Request(_) => {
                                        match app.request_tab {
                                            RequestTab::Headers => {
                                                app.headers_textarea.input(key);
                                            }
                                            RequestTab::Params => {
                                                app.params_textarea.input(key);
                                            }
                                            RequestTab::Body => {
                                                app.body_textarea.input(key);
                                            }
                                        }
                                    }
                                    SidebarSelection::Environment(_) => {
                                        app.env_textarea.input(key);
                                    }
                                }
                            }
                            SidebarMode::History => {
                                // Locked - do not forward keyboard inputs to request textareas in history mode
                            }
                        }
                        app.save_current_request();
                    }
                    Focus::Response => match key.code {
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.response_scroll = app.response_scroll.saturating_add(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.response_scroll = app.response_scroll.saturating_sub(1);
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
