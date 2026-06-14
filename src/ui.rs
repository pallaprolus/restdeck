use crate::app::{App, Focus, RequestTab, SidebarSelection, HTTP_METHODS};
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
    // 1. Render Sidebar (Split into Collections and Environments)
    // -------------------------------------------------------------
    let sidebar_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65),
            Constraint::Percentage(35),
        ])
        .split(sidebar_area);

    let collections_area = sidebar_layout[0];
    let environments_area = sidebar_layout[1];

    let selection = app.get_sidebar_selection();

    // 1.1 Render Collections
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
    f.render_widget(collections_list, collections_area);

    // 1.2 Render Environments
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
    f.render_widget(envs_list, environments_area);

    // -------------------------------------------------------------
    // 2. Render Config Panel / Environment Editor (Middle Panel)
    // -------------------------------------------------------------
    match selection {
        SidebarSelection::Request(_) => {
            // URL (Height 3), Tabs header (Height 3), Tab Content (Min 0)
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

            // URL Input & Method selector
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

            let tabs_line = Line::from(tab_spans);
            let tabs_para = Paragraph::new(tabs_line)
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
            // Render environment variables editor
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

    let status_span = match &app.response_status {
        Some(status) if status.starts_with('2') => {
            Span::styled(format!("STATUS: {}  ", status), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        }
        Some(status) => {
            Span::styled(format!("STATUS: {}  ", status), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        }
        None => Span::styled("STATUS: ---  ", Style::default().fg(Color::DarkGray)),
    };

    let time_span = match &app.response_time {
        Some(t) => Span::styled(format!("TIME: {}  ", t), Style::default().fg(Color::Yellow)),
        None => Span::styled("TIME: ---  ", Style::default().fg(Color::DarkGray)),
    };

    let size_span = match &app.response_size {
        Some(s) => Span::styled(format!("SIZE: {}", s), Style::default().fg(Color::Blue)),
        None => Span::styled("SIZE: ---", Style::default().fg(Color::DarkGray)),
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
        let raw_lines = app.response_content.lines();
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
        Focus::Sidebar => match selection {
            SidebarSelection::Request(_) => "➔ Nav: [Up/Down] | Select: [Enter] | Switch Pane: [Tab] | Quit: [Esc] / [Ctrl-C]",
            SidebarSelection::Environment(_) => "➔ Nav: [Up/Down] | Activate: [Enter] | Edit Env: [Tab] | Quit: [Esc] / [Ctrl-C]",
        },
        Focus::RequestUrl => "➔ Edit URL | Cycle Method: [Ctrl-M] | Send: [Ctrl-E] | Switch Pane: [Tab]",
        Focus::RequestTabContent => match selection {
            SidebarSelection::Request(_) => match app.request_tab {
                RequestTab::Headers => "➔ Edit Headers (Key: Value) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
                RequestTab::Params => "➔ Edit Parameters (key=value) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
                RequestTab::Body => "➔ Edit Request Body (JSON) | Switch Tabs: [Ctrl-H/P/B] | Send: [Ctrl-E] | Switch Pane: [Tab]",
            },
            SidebarSelection::Environment(_) => "➔ Edit Environment (key=value) | Switch Pane: [Tab]",
        },
        Focus::Response => "➔ View Response | Scroll: [Up/Down] or [j/k] | Switch Pane: [Tab]",
    };

    let env_name_str = match app.active_env_index {
        Some(idx) if idx < app.environments.len() => &app.environments[idx].name,
        _ => "None",
    };

    let status_line = Line::from(vec![
        Span::styled(" RestDeck v0.1.0 ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" [ENV: {}] ", env_name_str), Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" | {}", help_text), Style::default().bg(Color::DarkGray).fg(Color::White)),
    ]);

    f.render_widget(Paragraph::new(status_line), status_area);
}
