use crate::app::{App, Focus, RequestTab, SidebarMode, SidebarSelection, HTTP_METHODS};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

fn get_method_style(method: &str) -> Style {
    match method {
        "GET" => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        "POST" => Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
        "PUT" => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        "DELETE" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        "PATCH" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        _ => Style::default().fg(Color::White),
    }
}

fn get_border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    let main_area = chunks[0];
    let status_area = chunks[1];

    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(35),
            Constraint::Percentage(35),
        ])
        .split(main_area);

    let sidebar_area = main_cols[0];
    let request_area = main_cols[1];
    let response_area = main_cols[2];

    // -------------------------------------------------------------
    // 1. Render Sidebar (Split into Mode Tabs and Content Pane)
    // -------------------------------------------------------------
    let sidebar_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Sidebar Tabs
            Constraint::Min(0),    // Sidebar Content
        ])
        .split(sidebar_area);

    let sidebar_tabs_area = sidebar_layout[0];
    let sidebar_content_area = sidebar_layout[1];

    // Render Sidebar Tabs
    let collections_style = if app.sidebar_mode == SidebarMode::Collections {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let history_style = if app.sidebar_mode == SidebarMode::History {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let sidebar_tabs_para = Paragraph::new(Line::from(vec![
        Span::styled(" [Collections] ", collections_style),
        Span::styled("   [History] ", history_style),
    ]))
    .alignment(ratatui::layout::Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Mode "),
    );
    f.render_widget(sidebar_tabs_para, sidebar_tabs_area);

    // Sidebar Content Pane depending on Mode
    match app.sidebar_mode {
        SidebarMode::Collections => {
            let inner_sidebar_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(65),
                    Constraint::Percentage(35),
                ])
                .split(sidebar_content_area);

            let collections_sub_area = inner_sidebar_layout[0];
            let environments_sub_area = inner_sidebar_layout[1];

            let selection = app.get_sidebar_selection();

            // Render Collections
            let is_collections_focused = app.focus == Focus::Sidebar && matches!(selection, SidebarSelection::Request(_));
            let collections_border_style = get_border_style(is_collections_focused);

            let collections_items: Vec<ListItem> = app
                .collections
                .iter()
                .enumerate()
                .map(|(idx, req)| {
                    let method_span = match req.method.as_str() {
                        "GET" => Span::styled(" GET ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
                        "POST" => Span::styled(" POST", Style::default().bg(Color::LightMagenta).fg(Color::Black).add_modifier(Modifier::BOLD)),
                        "PUT" => Span::styled(" PUT ", Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)),
                        "DELETE" => Span::styled(" DEL ", Style::default().bg(Color::Red).fg(Color::Black).add_modifier(Modifier::BOLD)),
                        "PATCH" => Span::styled(" PAT ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
                        _ => Span::styled(" REQ ", Style::default().bg(Color::White).fg(Color::Black)),
                    };

                    let is_selected = matches!(selection, SidebarSelection::Request(i) if i == idx);
                    let name_span = if is_selected {
                        Span::styled(format!("  {}", req.name), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled(format!("  {}", req.name), Style::default().fg(Color::White))
                    };

                    let mut spans = vec![method_span, name_span];
                    if is_selected {
                        spans.insert(0, Span::styled("➔ ", Style::default().fg(Color::Cyan)));
                    } else {
                        spans.insert(0, Span::raw("  "));
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let collections_list = List::new(collections_items)
                .block(
                    Block::default()
                        .title(" 📁 Collections ")
                        .borders(Borders::ALL)
                        .border_style(collections_border_style),
                );
            f.render_widget(collections_list, collections_sub_area);

            // Render Environments
            let is_envs_focused = app.focus == Focus::Sidebar && matches!(selection, SidebarSelection::Environment(_));
            let envs_border_style = get_border_style(is_envs_focused);

            let envs_items: Vec<ListItem> = app
                .environments
                .iter()
                .enumerate()
                .map(|(idx, env)| {
                    let is_active = app.active_env_index == Some(idx);
                    let is_selected = matches!(selection, SidebarSelection::Environment(i) if i == idx);

                    let active_span = if is_active {
                        Span::styled("● ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("○ ", Style::default().fg(Color::DarkGray))
                    };

                    let name_span = if is_selected {
                        Span::styled(env.name.clone(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled(env.name.clone(), Style::default().fg(Color::White))
                    };

                    let mut spans = vec![active_span, name_span];
                    if is_selected {
                        spans.insert(0, Span::styled("➔ ", Style::default().fg(Color::Cyan)));
                    } else {
                        spans.insert(0, Span::raw("  "));
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let envs_list = List::new(envs_items)
                .block(
                    Block::default()
                        .title(" ⚙ Environments ")
                        .borders(Borders::ALL)
                        .border_style(envs_border_style),
                );
            f.render_widget(envs_list, environments_sub_area);
        }
        SidebarMode::History => {
            // Render History List
            let is_history_focused = app.focus == Focus::Sidebar;
            let history_border_style = get_border_style(is_history_focused);

            let history_items: Vec<ListItem> = app
                .history
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let is_selected = idx == app.history_index;

                    let status_span = match &item.response_status {
                        Some(status) if status.starts_with('2') => {
                            Span::styled(format!(" {} ", &status[..3]), Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
                        }
                        Some(status) => {
                            let display_status = if status.len() >= 3 { &status[..3] } else { "ERR" };
                            Span::styled(format!(" {} ", display_status), Style::default().bg(Color::Red).fg(Color::Black).add_modifier(Modifier::BOLD))
                        }
                        None => Span::styled(" --- ", Style::default().bg(Color::DarkGray).fg(Color::Black)),
                    };

                    let method_span = match item.method.as_str() {
                        "GET" => Span::styled(" GET", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        "POST" => Span::styled(" POST", Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD)),
                        "PUT" => Span::styled(" PUT", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        "DELETE" => Span::styled(" DEL", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                        _ => Span::styled(" REQ", Style::default().fg(Color::White)),
                    };

                    // Format URL / path
                    let url_str = if item.url.len() > 15 {
                        let path_part = item.url.find("//").map(|pos| &item.url[pos+2..]).unwrap_or(&item.url);
                        let sub_path = path_part.find('/').map(|pos| &path_part[pos..]).unwrap_or(path_part);
                        if sub_path.len() > 12 { format!("..{}", &sub_path[sub_path.len()-10..]) } else { sub_path.to_string() }
                    } else {
                        item.url.clone()
                    };

                    let url_span = if is_selected {
                        Span::styled(format!(" {}", url_str), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled(format!(" {}", url_str), Style::default().fg(Color::White))
                    };

                    let time_span = Span::styled(format!(" ({})", item.timestamp), Style::default().fg(Color::DarkGray));

                    let mut spans = vec![status_span, method_span, url_span, time_span];
                    if is_selected {
                        spans.insert(0, Span::styled("➔ ", Style::default().fg(Color::Cyan)));
                    } else {
                        spans.insert(0, Span::raw("  "));
                    }

                    ListItem::new(Line::from(spans))
                })
                .collect();

            let history_list = List::new(history_items)
                .block(
                    Block::default()
                        .title(" ⏳ Run History ")
                        .borders(Borders::ALL)
                        .border_style(history_border_style),
                );
            f.render_widget(history_list, sidebar_content_area);
        }
    }

    // -------------------------------------------------------------
    // 2. Render Config Panel / Environment Editor (Middle Panel)
    // -------------------------------------------------------------
    let selection = app.get_sidebar_selection();

    match app.sidebar_mode {
        SidebarMode::Collections => {
            match selection {
                SidebarSelection::Request(_) => {
                    let request_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),
                            Constraint::Length(3),
                            Constraint::Min(0),
                        ])
                        .split(request_area);

                    let url_area = request_layout[0];
                    let tabs_header_area = request_layout[1];
                    let tab_content_area = request_layout[2];

                    let is_url_focused = app.focus == Focus::RequestUrl;
                    let url_border_style = get_border_style(is_url_focused);

                    let url_row_split = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(12),
                            Constraint::Min(0),
                        ])
                        .split(url_area);

                    let method_badge_area = url_row_split[0];
                    let url_input_area = url_row_split[1];

                    let method_str = HTTP_METHODS[app.method_index];
                    let method_para = Paragraph::new(method_str)
                        .style(get_method_style(method_str))
                        .alignment(ratatui::layout::Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(url_border_style)
                                .title(" Method "),
                        );
                    f.render_widget(method_para, method_badge_area);

                    app.url_textarea.set_block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(url_border_style)
                            .title(" URL "),
                    );
                    f.render_widget(app.url_textarea.widget(), url_input_area);

                    // Tabs Header
                    let is_tab_content_focused = app.focus == Focus::RequestTabContent;
                    let tab_content_border_style = get_border_style(is_tab_content_focused);

                    let tab_titles = vec![" [Headers] ", " [Params] ", " [Body] "];
                    let active_tab_index = match app.request_tab {
                        RequestTab::Headers => 0,
                        RequestTab::Params => 1,
                        RequestTab::Body => 2,
                    };

                    let tab_spans: Vec<Span> = tab_titles
                        .iter()
                        .enumerate()
                        .map(|(i, &title)| {
                            if i == active_tab_index {
                                Span::styled(
                                    title,
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD)
                                        .add_modifier(Modifier::UNDERLINED),
                                )
                            } else {
                                Span::styled(title, Style::default().fg(Color::DarkGray))
                            }
                        })
                        .collect();

                    let tabs_para = Paragraph::new(Line::from(tab_spans))
                        .alignment(ratatui::layout::Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::DarkGray))
                                .title(" Config Tabs "),
                        );
                    f.render_widget(tabs_para, tabs_header_area);

                    // Render Tab Content
                    match app.request_tab {
                        RequestTab::Headers => {
                            app.headers_textarea.set_block(
                                Block::default()
                                    .title(" HTTP Headers (Key: Value) ")
                                    .borders(Borders::ALL)
                                    .border_style(tab_content_border_style),
                            );
                            f.render_widget(app.headers_textarea.widget(), tab_content_area);
                        }
                        RequestTab::Params => {
                            app.params_textarea.set_block(
                                Block::default()
                                    .title(" Query Parameters (key=value) ")
                                    .borders(Borders::ALL)
                                    .border_style(tab_content_border_style),
                            );
                            f.render_widget(app.params_textarea.widget(), tab_content_area);
                        }
                        RequestTab::Body => {
                            app.body_textarea.set_block(
                                Block::default()
                                    .title(" Request Body (JSON/Raw) ")
                                    .borders(Borders::ALL)
                                    .border_style(tab_content_border_style),
                            );
                            f.render_widget(app.body_textarea.widget(), tab_content_area);
                        }
                    }
                }
                SidebarSelection::Environment(idx) => {
                    let is_env_focused = app.focus == Focus::RequestTabContent;
                    let env_border_style = get_border_style(is_env_focused);
                    let env_name = &app.environments[idx].name;

                    app.env_textarea.set_block(
                        Block::default()
                            .title(format!(" Edit Environment Variables: {} (key=value) ", env_name))
                            .borders(Borders::ALL)
                            .border_style(env_border_style),
                    );
                    f.render_widget(app.env_textarea.widget(), request_area);
                }
            }
        }
        SidebarMode::History => {
            // In History mode, display a locked/view-only request configuration pane
            // or just render the editor layout containing the historical details.
            // Rendering the textareas is highly useful because it shows the exact URL/headers that ran!
            let request_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(request_area);

            let url_area = request_layout[0];
            let tabs_header_area = request_layout[1];
            let tab_content_area = request_layout[2];

            let is_url_focused = app.focus == Focus::RequestUrl;
            let url_border_style = get_border_style(is_url_focused);

            let url_row_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(12),
                    Constraint::Min(0),
                ])
                .split(url_area);

            let method_badge_area = url_row_split[0];
            let url_input_area = url_row_split[1];

            // Render URL/method from active textareas (updated by history_index change in main.rs)
            let method_str = HTTP_METHODS[app.method_index];
            let method_para = Paragraph::new(method_str)
                .style(get_method_style(method_str))
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(url_border_style)
                        .title(" Method "),
                );
            f.render_widget(method_para, method_badge_area);

            app.url_textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(url_border_style)
                    .title(" URL (History - Press Enter to Restore) "),
            );
            f.render_widget(app.url_textarea.widget(), url_input_area);

            // Tab headers
            let is_tab_content_focused = app.focus == Focus::RequestTabContent;
            let tab_content_border_style = get_border_style(is_tab_content_focused);

            let tab_titles = vec![" [Headers] ", " [Params] ", " [Body] "];
            let active_tab_index = match app.request_tab {
                RequestTab::Headers => 0,
                RequestTab::Params => 1,
                RequestTab::Body => 2,
            };

            let tab_spans: Vec<Span> = tab_titles
                .iter()
                .enumerate()
                .map(|(i, &title)| {
                    if i == active_tab_index {
                        Span::styled(
                            title,
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                                .add_modifier(Modifier::UNDERLINED),
                        )
                    } else {
                        Span::styled(title, Style::default().fg(Color::DarkGray))
                    }
                })
                .collect();

            let tabs_para = Paragraph::new(Line::from(tab_spans))
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Config Tabs "),
                );
            f.render_widget(tabs_para, tabs_header_area);

            // Tab contents
            match app.request_tab {
                RequestTab::Headers => {
                    app.headers_textarea.set_block(
                        Block::default()
                            .title(" HTTP Headers (History) ")
                            .borders(Borders::ALL)
                            .border_style(tab_content_border_style),
                    );
                    f.render_widget(app.headers_textarea.widget(), tab_content_area);
                }
                RequestTab::Params => {
                    app.params_textarea.set_block(
                        Block::default()
                            .title(" Query Parameters (History) ")
                            .borders(Borders::ALL)
                            .border_style(tab_content_border_style),
                    );
                    f.render_widget(app.params_textarea.widget(), tab_content_area);
                }
                RequestTab::Body => {
                    app.body_textarea.set_block(
                        Block::default()
                            .title(" Request Body (History) ")
                            .borders(Borders::ALL)
                            .border_style(tab_content_border_style),
                    );
                    f.render_widget(app.body_textarea.widget(), tab_content_area);
                }
            }
        }
    }

    // -------------------------------------------------------------
    // 3. Render Response Pane
    // -------------------------------------------------------------
    let is_response_focused = app.focus == Focus::Response;
    let response_border_style = get_border_style(is_response_focused);

    let response_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(response_area);

    let resp_meta_area = response_layout[0];
    let resp_body_area = response_layout[1];

    // Determine metadata and body source (TUI active state or highlighted HistoryItem)
    let (status_str, time_str, size_str, body_str) = if app.sidebar_mode == SidebarMode::History && !app.history.is_empty() && app.history_index < app.history.len() {
        let item = &app.history[app.history_index];
        (
            item.response_status.clone().unwrap_or_else(|| "---".to_string()),
            item.response_time.clone().unwrap_or_else(|| "---".to_string()),
            item.response_size.clone().unwrap_or_else(|| "---".to_string()),
            &item.response_content,
        )
    } else {
        (
            app.response_status.clone().unwrap_or_else(|| "---".to_string()),
            app.response_time.clone().unwrap_or_else(|| "---".to_string()),
            app.response_size.clone().unwrap_or_else(|| "---".to_string()),
            &app.response_content,
        )
    };

    let status_span = if status_str.starts_with('2') {
        Span::styled(format!("STATUS: {}  ", status_str), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else if status_str != "---" && status_str != "Error" {
        Span::styled(format!("STATUS: {}  ", status_str), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(format!("STATUS: {}  ", status_str), Style::default().fg(Color::DarkGray))
    };

    let time_span = if time_str != "---" {
        Span::styled(format!("TIME: {}  ", time_str), Style::default().fg(Color::Yellow))
    } else {
        Span::styled(format!("TIME: {}  ", time_str), Style::default().fg(Color::DarkGray))
    };

    let size_span = if size_str != "---" {
        Span::styled(format!("SIZE: {}", size_str), Style::default().fg(Color::Blue))
    } else {
        Span::styled(format!("SIZE: {}", size_str), Style::default().fg(Color::DarkGray))
    };

    let meta_line = Line::from(vec![status_span, time_span, size_span]);
    let meta_para = Paragraph::new(meta_line)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Response Metadata "),
        );
    f.render_widget(meta_para, resp_meta_area);

    let response_widget = if app.is_loading {
        Paragraph::new("\n\n⏳ Sending Request...")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Yellow))
    } else {
        let raw_lines = body_str.lines();
        let mut text_lines = Vec::new();
        
        for line in raw_lines {
            let mut spans = Vec::new();
            let trimmed = line.trim_start();
            
            if (trimmed.starts_with('"') && trimmed.contains(':')) || trimmed.starts_with('{') || trimmed.starts_with('}') || trimmed.starts_with('[') || trimmed.starts_with(']') {
                if let Some(colon_pos) = line.find(':') {
                    let key_part = &line[..colon_pos];
                    let val_part = &line[colon_pos..];
                    
                    spans.push(Span::styled(key_part, Style::default().fg(Color::LightBlue)));
                    
                    if val_part.contains("true") || val_part.contains("false") {
                        spans.push(Span::styled(val_part, Style::default().fg(Color::LightGreen)));
                    } else if val_part.chars().any(|c| c.is_numeric()) {
                        spans.push(Span::styled(val_part, Style::default().fg(Color::Yellow)));
                    } else if val_part.contains('"') {
                        spans.push(Span::styled(val_part, Style::default().fg(Color::LightRed)));
                    } else {
                        spans.push(Span::styled(val_part, Style::default().fg(Color::White)));
                    }
                } else {
                    spans.push(Span::styled(line, Style::default().fg(Color::White)));
                }
            } else {
                spans.push(Span::styled(line, Style::default().fg(Color::DarkGray)));
            }
            text_lines.push(Line::from(spans));
        }

        let total_lines = text_lines.len();
        let viewport_height = resp_body_area.height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(viewport_height) as u16;
        if app.response_scroll > max_scroll {
            app.response_scroll = max_scroll;
        }

        Paragraph::new(Text::from(text_lines))
            .scroll((app.response_scroll, 0))
            .wrap(Wrap { trim: false })
    };

    let resp_block = Block::default()
        .title(" 📥 Response ")
        .borders(Borders::ALL)
        .border_style(response_border_style);

    f.render_widget(response_widget.block(resp_block), resp_body_area);

    // -------------------------------------------------------------
    // 4. Render Status Bar
    // -------------------------------------------------------------
    let help_text = match app.focus {
        Focus::Sidebar => match app.sidebar_mode {
            SidebarMode::Collections => match selection {
                SidebarSelection::Request(_) => "➔ Nav: [Up/Down] | Select: [Enter] | Switch Mode: [Ctrl-Y] | Switch Pane: [Tab] | Quit: [Esc]",
                SidebarSelection::Environment(_) => "➔ Nav: [Up/Down] | Activate: [Enter] | Switch Mode: [Ctrl-Y] | Edit Env: [Tab] | Quit: [Esc]",
            },
            SidebarMode::History => "➔ Nav: [Up/Down] | Restore Request: [Enter] | Switch Mode: [Ctrl-Y] | Switch Pane: [Tab]",
        },
        Focus::RequestUrl => "➔ Edit URL | Cycle Method: [Ctrl-M] | Send: [Ctrl-E] | Switch Pane: [Tab]",
        Focus::RequestTabContent => match app.sidebar_mode {
            SidebarMode::Collections => match selection {
                SidebarSelection::Request(_) => match app.request_tab {
                    RequestTab::Headers => "➔ Edit Headers (Key: Value) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
                    RequestTab::Params => "➔ Edit Parameters (key=value) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
                    RequestTab::Body => "➔ Edit Request Body (JSON) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
                },
                SidebarSelection::Environment(_) => "➔ Edit Environment (key=value) | Switch Pane: [Tab]",
            },
            SidebarMode::History => "➔ View Request Configuration (History Mode) | Switch Pane: [Tab]",
        },
        Focus::Response => "➔ View Response | Scroll: [Up/Down] or [j/k] | Switch Pane: [Tab]",
    };

    let env_name_str = match app.active_env_index {
        Some(idx) if idx < app.environments.len() => &app.environments[idx].name,
        _ => "None",
    };

    let mode_str = match app.sidebar_mode {
        SidebarMode::Collections => "COLLECTIONS",
        SidebarMode::History => "HISTORY",
    };

    let status_line = Line::from(vec![
        Span::styled(" RestDeck v0.1.0 ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" [{}] ", mode_str), Style::default().bg(Color::LightMagenta).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" [ENV: {}] ", env_name_str), Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" | {}", help_text), Style::default().bg(Color::DarkGray).fg(Color::White)),
    ]);

    f.render_widget(Paragraph::new(status_line), status_area);
}
