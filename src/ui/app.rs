use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Terminal;

use crate::composer::exec::kill_process;
use crate::composer::{self, Runner, StreamLine};
use crate::security;
use crate::ui::components::*;
use crate::ui::layout::{compute_layout, COLLAPSED_PANEL_H};
use crate::ui::messages::Action;
use crate::ui::panels;
use crate::ui::style::{styles, theme};

const TAB_PACKAGES: usize = 0;
const TAB_AUDIT: usize = 1;
const TAB_PROJECT: usize = 2;

/// Messages from background threads.
enum BgMsg {
    PackagesLoaded {
        packages: Vec<composer::Package>,
        lock_hash: String,
        framework: Option<composer::FrameworkInfo>,
        composer_json: Box<composer::ComposerJSON>,
    },
    PackagesError(String),
    OutdatedLoaded(composer::OutdatedResult),
    OutdatedError(String),
    AuditLoaded(composer::AuditResult),
    AuditError(String),
    ShowLoaded(Box<composer::ShowResult>),
    ShowError(String),
    RestrictedVersionLoaded {
        name: String,
        best_version: Option<String>,
    },
    WhyLoaded {
        pkg: String,
        entries: Vec<composer::WhyEntry>,
    },
    ComposerInfo {
        version: String,
        path: String,
    },
}

pub struct App {
    dir: String,
    _parser: composer::Parser,
    runner: std::sync::Arc<Runner>,
    layout: crate::ui::layout::Layout,

    // Panels
    packages: panels::PackagesPanel,
    audit: panels::AuditPanel,
    output: panels::OutputPanel,

    // Components
    _status_bar: StatusBar,
    confirm: ConfirmDialog,
    choice: ChoiceDialog,
    help: HelpPopup,
    input: InputBox,
    spinner: LoadingSpinner,

    // State
    composer_info: String,
    stream_rx: Option<mpsc::Receiver<StreamLine>>,
    child_pid: Option<u32>,
    bg_rx: mpsc::Receiver<BgMsg>,
    bg_tx: mpsc::Sender<BgMsg>,
    active_tab: usize,
    loading: bool,
    err: Option<String>,
    outdated_result: Vec<composer::OutdatedPackage>,
    audit_result: Option<composer::AuditResult>,
    lock_hash: String,
    pending_action: Option<PendingAction>,
    pending_upgrade_target: Option<String>,
    pending_upgrade_dev: bool,
    show_result: Option<composer::ShowResult>,
    loading_show: bool,
    detail_focus: bool,
    detail_scroll: u16,

    // Framework
    framework_info: Option<composer::FrameworkInfo>,

    // Project metadata
    composer_json: Option<composer::ComposerJSON>,

    // Loading states
    loading_packages: bool,
    loading_outdated: bool,
    loading_audit: bool,
    resolving_restricted: std::collections::HashSet<String>,
    resolving_why: std::collections::HashSet<String>,

    // Mouse hit zones (updated each render)
    hit_tab_bar: Rect,
    hit_prod_inner: Rect,
    hit_dev_inner: Rect,
    hit_audit_inner: Rect,
}

enum PendingAction {
    Remove(String),
    Update(String),
    UpdateAll,
}

impl App {
    pub fn new(dir: String, runner: Runner, _version: String) -> Self {
        let (bg_tx, bg_rx) = mpsc::channel();

        App {
            dir,
            _parser: composer::Parser::new(),
            runner: std::sync::Arc::new(runner),
            layout: crate::ui::layout::Layout::default(),
            packages: panels::PackagesPanel::new(),
            audit: panels::AuditPanel::new(),
            output: panels::OutputPanel::new(),
            _status_bar: StatusBar::new(),
            confirm: ConfirmDialog::new(),
            choice: ChoiceDialog::new(),
            help: HelpPopup::new(),
            input: InputBox::new(),
            spinner: LoadingSpinner::new(),
            composer_info: String::new(),
            stream_rx: None,
            child_pid: None,
            bg_rx,
            bg_tx,
            active_tab: TAB_PACKAGES,
            loading: false,
            err: None,
            outdated_result: vec![],
            audit_result: None,
            lock_hash: String::new(),
            pending_action: None,
            pending_upgrade_target: None,
            pending_upgrade_dev: false,
            show_result: None,
            loading_show: false,
            detail_focus: false,
            detail_scroll: 0,
            framework_info: None,
            composer_json: None,
            loading_packages: false,
            loading_outdated: false,
            loading_audit: false,
            resolving_restricted: std::collections::HashSet::new(),
            resolving_why: std::collections::HashSet::new(),
            hit_tab_bar: Rect::default(),
            hit_prod_inner: Rect::default(),
            hit_dev_inner: Rect::default(),
            hit_audit_inner: Rect::default(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Async initial data load — all in parallel
        self.spawn_load_packages();
        self.spawn_load_outdated();
        self.spawn_load_audit();
        self.spawn_load_composer_info();

        loop {
            terminal.draw(|f| self.render(f))?;

            // Process background messages (non-blocking)
            self.process_bg_messages();

            // Check streaming channel — drain all available messages
            if let Some(rx) = &self.stream_rx {
                let mut got_done = false;
                while let Ok(line) = rx.try_recv() {
                    if line.done {
                        if let Some(err_msg) = line.err {
                            self.output.append_line(&format!("Error: {err_msg}"));
                        } else {
                            self.output.append_line("✓ Done");
                        }
                        got_done = true;
                        break;
                    } else {
                        self.output.append_line(&line.text);
                    }
                }
                if got_done {
                    self.loading = false;
                    self.stream_rx = None;
                    self.child_pid = None;
                    self.lock_hash.clear();
                    self.spawn_load_packages();
                    self.spawn_load_outdated();
                    self.spawn_load_audit();
                }
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        if self.handle_key(key) {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse);
                    }
                    Event::Resize(w, h) => {
                        self.layout = compute_layout(w, h);
                    }
                    _ => {}
                }
            }

            self.spinner.tick();
        }

        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            crossterm::event::DisableMouseCapture,
            crossterm::terminal::LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn process_bg_messages(&mut self) {
        while let Ok(msg) = self.bg_rx.try_recv() {
            match msg {
                BgMsg::PackagesLoaded {
                    packages,
                    lock_hash,
                    framework,
                    composer_json,
                } => {
                    self.loading_packages = false;
                    self.framework_info = framework;
                    self.composer_json = Some(*composer_json);
                    self.packages.set_packages(packages);
                    self.packages.update_statuses(
                        Some(&self.outdated_result),
                        self.audit_result.as_ref(),
                        self.framework_info.as_ref(),
                    );
                    self.audit
                        .update_versions(&self.packages.packages, &self.outdated_result);
                    self.spawn_load_why_for_transitive();
                    // On reload (after an action), re-fetch outdated/audit if lock changed
                    if !self.lock_hash.is_empty() && lock_hash != self.lock_hash {
                        self.spawn_load_outdated();
                        self.spawn_load_audit();
                    }
                    self.lock_hash = lock_hash;
                    if !self.loading_outdated && !self.loading_audit {
                        self.spinner.stop();
                    }
                }
                BgMsg::PackagesError(e) => {
                    self.loading_packages = false;
                    self.err = Some(e);
                    self.spinner.stop();
                }
                BgMsg::OutdatedLoaded(result) => {
                    self.loading_outdated = false;
                    self.outdated_result = result.installed;
                    self.packages.update_statuses(
                        Some(&self.outdated_result),
                        self.audit_result.as_ref(),
                        self.framework_info.as_ref(),
                    );
                    self.audit
                        .update_versions(&self.packages.packages, &self.outdated_result);
                    self.spawn_resolve_all_restricted();
                    if !self.loading_packages && !self.loading_audit {
                        self.spinner.stop();
                    }
                }
                BgMsg::OutdatedError(e) => {
                    self.loading_outdated = false;
                    self.err = Some(e);
                    if !self.loading_packages && !self.loading_audit {
                        self.spinner.stop();
                    }
                }
                BgMsg::AuditLoaded(result) => {
                    self.loading_audit = false;
                    self.audit.set_audit(Some(&result));
                    self.audit_result = Some(result);
                    self.audit
                        .update_versions(&self.packages.packages, &self.outdated_result);
                    self.spawn_load_why_for_transitive();
                    self.packages.update_statuses(
                        Some(&self.outdated_result),
                        self.audit_result.as_ref(),
                        self.framework_info.as_ref(),
                    );
                    if !self.loading_packages && !self.loading_outdated {
                        self.spinner.stop();
                    }
                }
                BgMsg::AuditError(e) => {
                    self.loading_audit = false;
                    self.err = Some(e);
                    if !self.loading_packages && !self.loading_outdated {
                        self.spinner.stop();
                    }
                }
                BgMsg::ShowLoaded(result) => {
                    self.loading_show = false;
                    self.show_result = Some(*result);
                    self.detail_focus = true;
                    self.detail_scroll = 0;
                    self.spinner.stop();
                }
                BgMsg::ShowError(e) => {
                    self.loading_show = false;
                    self.err = Some(e);
                    self.spinner.stop();
                }
                BgMsg::RestrictedVersionLoaded { name, best_version } => {
                    self.resolving_restricted.remove(&name);
                    self.packages.resolve_restricted(&name, best_version);
                }
                BgMsg::WhyLoaded { pkg, entries } => {
                    self.resolving_why.remove(&pkg);
                    self.audit.set_why_result(&pkg, entries);
                }
                BgMsg::ComposerInfo { version, path } => {
                    self.composer_info = format!("composer {version} ({path})");
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Overlays first (in priority order)
        if self.confirm.is_visible() {
            if let Some(_action) = self.confirm.handle_key(key) {
                if self.confirm.confirmed {
                    if let Some(pending) = self.pending_action.take() {
                        match pending {
                            PendingAction::Remove(name) => self.exec_remove(&name),
                            PendingAction::Update(name) => self.exec_update(&name),
                            PendingAction::UpdateAll => self.exec_update(""),
                        }
                    }
                } else {
                    self.pending_action = None;
                }
            }
            return false;
        }

        if self.choice.is_visible() {
            if let Some(selected) = self.choice.handle_key(key) {
                if selected != '\x1b' {
                    match selected {
                        'u' => {
                            if let Some(PendingAction::Update(name)) = self.pending_action.take() {
                                self.pending_upgrade_target = None;
                                self.exec_update(&name);
                            }
                        }
                        'U' => {
                            self.pending_action = None;
                            let dev = self.pending_upgrade_dev;
                            if let Some(target) = self.pending_upgrade_target.take() {
                                self.exec_upgrade(&target, dev);
                            }
                        }
                        _ => {}
                    }
                } else {
                    self.pending_action = None;
                    self.pending_upgrade_target = None;
                }
            }
            return false;
        }

        if self.input.is_visible() {
            if let Some(Action::InputSubmit(value)) = self.input.handle_key(key) {
                self.handle_require(&value);
            }
            return false;
        }

        if self.help.is_visible() {
            self.help.handle_key(key);
            return false;
        }

        // Ctrl+C kills the running process
        if self.loading
            && key.code == KeyCode::Char('c')
            && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            if let Some(pid) = self.child_pid.take() {
                kill_process(pid);
            }
            self.stream_rx = None;
            self.loading = false;
            self.output.append_line("⊘ Cancelled");
            return false;
        }

        if self.output.is_visible() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if !self.loading {
                        self.output.hide();
                    }
                    return false; // absorb q/Esc so we don't quit
                }
                _ => {} // let other keys fall through to normal handling
            }
        }

        // Filter mode in packages panel
        if self.active_tab == TAB_PACKAGES && self.packages.is_filtering() {
            self.packages.handle_key(key);
            return false;
        }

        // Detail panel focus mode — scroll with j/k/Up/Down, leave with Esc/h
        if self.detail_focus {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Char('q') => return true,
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                    if self.show_result.is_some() {
                        self.show_result = None;
                    }
                    self.detail_focus = false;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
                KeyCode::Char('g') => {
                    self.detail_scroll = 0;
                }
                KeyCode::Char('G') => {
                    self.detail_scroll = u16::MAX;
                }
                _ => {}
            }
            return false;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('q') => return true,
            KeyCode::Tab | KeyCode::Right => {
                if self.active_tab == TAB_PACKAGES && !self.packages.focus_dev {
                    // require → require-dev
                    self.packages.focus_dev = true;
                } else if self.active_tab == TAB_PACKAGES && self.packages.focus_dev {
                    // require-dev → audit
                    self.packages.focus_dev = false;
                    self.active_tab = TAB_AUDIT;
                    self.show_result = None;
                } else if self.active_tab == TAB_AUDIT {
                    // audit → project
                    self.active_tab = TAB_PROJECT;
                    self.show_result = None;
                } else {
                    // project → require
                    self.active_tab = TAB_PACKAGES;
                    self.packages.focus_dev = false;
                    self.show_result = None;
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                if self.active_tab == TAB_PACKAGES && self.packages.focus_dev {
                    // require-dev → require
                    self.packages.focus_dev = false;
                } else if self.active_tab == TAB_PACKAGES && !self.packages.focus_dev {
                    // require → project
                    self.active_tab = TAB_PROJECT;
                    self.show_result = None;
                } else if self.active_tab == TAB_AUDIT {
                    // audit → require-dev
                    self.active_tab = TAB_PACKAGES;
                    self.packages.focus_dev = true;
                    self.show_result = None;
                } else {
                    // project → audit
                    self.active_tab = TAB_AUDIT;
                    self.show_result = None;
                }
            }
            KeyCode::Char('r') if self.active_tab == TAB_PACKAGES => {
                self.input.show("Require Package", "vendor/package");
            }
            KeyCode::Char('d') if self.active_tab == TAB_PACKAGES => {
                if let Some(pkg) = self.packages.selected_package() {
                    let name = pkg.name.clone();
                    self.pending_action = Some(PendingAction::Remove(name.clone()));
                    self.confirm.show(&format!("Run `composer remove {name}`?"));
                }
            }
            KeyCode::Char('u') => self.handle_update_selected(),
            KeyCode::Char('U') => {
                self.pending_action = Some(PendingAction::UpdateAll);
                self.confirm.show("Run `composer update`?");
            }
            KeyCode::Char('s') | KeyCode::Enter if self.active_tab == TAB_PACKAGES => {
                self.handle_show_selected();
            }
            KeyCode::Enter | KeyCode::Char('l') if self.active_tab == TAB_AUDIT => {
                if self.audit.selected_entry().is_some() {
                    self.detail_focus = true;
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Char('l') if self.active_tab == TAB_PACKAGES => {
                if self.show_result.is_some() {
                    self.detail_focus = true;
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Char('o') if self.active_tab == TAB_PACKAGES => {
                self.packages.toggle_outdated_filter();
            }
            KeyCode::Char('R') => {
                self.spawn_load_packages();
                self.spawn_load_outdated();
                self.spawn_load_audit();
            }
            KeyCode::Char('?') => self.help.show(),
            KeyCode::Char('/') if self.active_tab == TAB_PACKAGES => {
                self.packages.start_filter();
            }
            _ => match self.active_tab {
                TAB_PACKAGES => self.packages.handle_key(key),
                TAB_AUDIT => self.audit.handle_key(key),
                _ => {}
            },
        }
        false
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        // Ignore mouse when dialogs are open
        if self.confirm.is_visible()
            || self.choice.is_visible()
            || self.help.is_visible()
            || self.input.is_visible()
            || self.output.is_visible()
        {
            return;
        }

        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Tab bar click
                if row >= self.hit_tab_bar.y && row < self.hit_tab_bar.y + self.hit_tab_bar.height {
                    if let Some(tab) = tab_index_at(col) {
                        self.detail_focus = false;
                        self.show_result = None;
                        match tab {
                            TAB_PACKAGES => {
                                self.active_tab = TAB_PACKAGES;
                                self.packages.focus_dev = false;
                            }
                            TAB_AUDIT => {
                                self.active_tab = TAB_AUDIT;
                            }
                            TAB_PROJECT => {
                                self.active_tab = TAB_PROJECT;
                            }
                            _ => {}
                        }
                    }
                    return;
                }

                // Prod list click
                if Self::in_rect(col, row, self.hit_prod_inner) && self.active_tab != TAB_PROJECT {
                    let rel_y = (row - self.hit_prod_inner.y) as usize;
                    let idx = self.packages.prod_scroll + rel_y;
                    if idx < self.packages.prod_items.len() {
                        self.active_tab = TAB_PACKAGES;
                        self.packages.focus_dev = false;
                        self.packages.prod_cursor = idx;
                        self.detail_focus = false;
                        self.show_result = None;
                    }
                    return;
                }

                // Dev list click
                if Self::in_rect(col, row, self.hit_dev_inner) && self.active_tab != TAB_PROJECT {
                    let rel_y = (row - self.hit_dev_inner.y) as usize;
                    let idx = self.packages.dev_scroll + rel_y;
                    if idx < self.packages.dev_items.len() {
                        self.active_tab = TAB_PACKAGES;
                        self.packages.focus_dev = true;
                        self.packages.dev_cursor = idx;
                        self.detail_focus = false;
                        self.show_result = None;
                    }
                    return;
                }

                // Audit list click
                if Self::in_rect(col, row, self.hit_audit_inner) && self.active_tab != TAB_PROJECT {
                    let rel_y = (row - self.hit_audit_inner.y) as usize;
                    let idx = self.audit.offset + rel_y;
                    if idx < self.audit.total_items() {
                        self.active_tab = TAB_AUDIT;
                        self.audit.cursor = idx;
                        self.detail_focus = false;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if Self::in_rect(col, row, self.hit_prod_inner) {
                    if self.packages.prod_cursor > 0 {
                        self.packages.prod_cursor -= 1;
                    }
                } else if Self::in_rect(col, row, self.hit_dev_inner) {
                    if self.packages.dev_cursor > 0 {
                        self.packages.dev_cursor -= 1;
                    }
                } else if Self::in_rect(col, row, self.hit_audit_inner) {
                    if self.audit.cursor > 0 {
                        self.audit.cursor -= 1;
                        if self.audit.cursor < self.audit.offset {
                            self.audit.offset = self.audit.cursor;
                        }
                    }
                } else if self.detail_focus {
                    self.detail_scroll = self.detail_scroll.saturating_sub(3);
                }
            }
            MouseEventKind::ScrollDown => {
                if Self::in_rect(col, row, self.hit_prod_inner) {
                    if self.packages.prod_cursor + 1 < self.packages.prod_items.len() {
                        self.packages.prod_cursor += 1;
                    }
                } else if Self::in_rect(col, row, self.hit_dev_inner) {
                    if self.packages.dev_cursor + 1 < self.packages.dev_items.len() {
                        self.packages.dev_cursor += 1;
                    }
                } else if Self::in_rect(col, row, self.hit_audit_inner) {
                    if self.audit.cursor + 1 < self.audit.total_items() {
                        self.audit.cursor += 1;
                    }
                } else if self.detail_focus {
                    self.detail_scroll = self.detail_scroll.saturating_add(3);
                }
            }
            _ => {}
        }
    }

    fn in_rect(col: u16, row: u16, rect: Rect) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let size = f.area();
        self.layout = compute_layout(size.width, size.height);

        // Tab bar at the top
        let tab_area = Rect::new(0, 0, size.width, TAB_BAR_H);
        self.hit_tab_bar = tab_area;
        render_tab_bar(self.active_tab, tab_area, f.buffer_mut());

        let content_area = Rect::new(
            0,
            TAB_BAR_H,
            size.width,
            self.layout.content_h.saturating_sub(TAB_BAR_H),
        );

        if self.help.is_visible() {
            self.render_panels(f, content_area);
            let dialog_area = centered_rect(60, 20, content_area);
            let text = self.help.view();
            let paragraph = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::COLOR_BORDER_FOCUS)),
            );
            f.render_widget(ratatui::widgets::Clear, dialog_area);
            f.render_widget(paragraph, dialog_area);
        } else if self.confirm.is_visible() {
            self.render_panels(f, content_area);
            let dialog_area = centered_rect(50, 7, content_area);
            let text = self.confirm.view();
            let paragraph = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::COLOR_BORDER_FOCUS)),
            );
            f.render_widget(ratatui::widgets::Clear, dialog_area);
            f.render_widget(paragraph, dialog_area);
        } else if self.choice.is_visible() {
            self.render_panels(f, content_area);
            let dialog_area = centered_rect(55, 10, content_area);
            let text = self.choice.view();
            let paragraph = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::COLOR_BORDER_FOCUS)),
            );
            f.render_widget(ratatui::widgets::Clear, dialog_area);
            f.render_widget(paragraph, dialog_area);
        } else if self.input.is_visible() {
            self.render_panels(f, content_area);
            let dialog_area = centered_rect(60, 7, content_area);
            let text = self.input.view();
            let paragraph = Paragraph::new(text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::COLOR_BORDER_FOCUS)),
            );
            f.render_widget(ratatui::widgets::Clear, dialog_area);
            f.render_widget(paragraph, dialog_area);
        } else {
            self.render_panels(f, content_area);
        }

        // --- Status bar ---
        let status_area = Rect::new(0, size.height.saturating_sub(1), size.width, 1);
        self.render_status_bar(status_area, f.buffer_mut());
    }

    fn render_panels(&mut self, f: &mut ratatui::Frame, area: Rect) {
        // Project tab: full-width panel
        if self.active_tab == TAB_PROJECT {
            if let Some(cj) = &self.composer_json {
                panels::project::render_project_panel(
                    cj,
                    self.framework_info.as_ref(),
                    area,
                    f.buffer_mut(),
                    true,
                );
            } else {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::COLOR_BORDER))
                    .title(" Project ");
                let inner = block.inner(area);
                block.render(area, f.buffer_mut());
                Paragraph::new("Loading…").render(inner, f.buffer_mut());
            }
            return;
        }

        // Split into left (stacked cards) and right (detail) columns
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.layout.left_width),
                Constraint::Min(0),
            ])
            .split(area);

        let left_area = chunks[0];
        let right_area = chunks[1];

        // --- Left column: 2 stacked cards (lazygit-style) ---
        // Active panel: 70% height, inactive panel: 30%
        let total_h = left_area.height;
        let active_h = (total_h * 7 / 10).max(COLLAPSED_PANEL_H);
        let inactive_h = total_h.saturating_sub(active_h);

        let panel_tabs = [TAB_PACKAGES, TAB_AUDIT];
        let mut y = left_area.y;

        for &tab in &panel_tabs {
            let is_active = tab == self.active_tab;
            let h = if is_active { active_h } else { inactive_h };
            let panel_area = Rect::new(left_area.x, y, left_area.width, h);

            match tab {
                TAB_PACKAGES => {
                    self.packages.set_size(panel_area.width, panel_area.height);
                    self.packages.render(panel_area, f.buffer_mut(), is_active);
                    // Store hit zones for prod/dev sub-panels (matches render logic)
                    let half_h = panel_area.height / 2;
                    let prod_area = Rect::new(panel_area.x, panel_area.y, panel_area.width, half_h);
                    let dev_area = Rect::new(
                        panel_area.x,
                        panel_area.y + half_h,
                        panel_area.width,
                        panel_area.height.saturating_sub(half_h),
                    );
                    let prod_block = Block::default().borders(Borders::ALL);
                    let dev_block = Block::default().borders(Borders::ALL);
                    self.hit_prod_inner = prod_block.inner(prod_area);
                    self.hit_dev_inner = dev_block.inner(dev_area);
                }
                TAB_AUDIT => {
                    self.audit.set_size(panel_area.width, panel_area.height);
                    self.audit.render(panel_area, f.buffer_mut(), is_active);
                    let audit_block = Block::default().borders(Borders::ALL);
                    self.hit_audit_inner = audit_block.inner(panel_area);
                }
                _ => {}
            }

            y += h;
        }

        // --- Right column: split into main area + output panel (when visible) ---
        let (main_right_area, output_bottom_area) = if self.output.is_visible() {
            let output_h = (right_area.height * 4 / 10).max(6).min(right_area.height);
            let main_h = right_area.height.saturating_sub(output_h);
            (
                Rect::new(right_area.x, right_area.y, right_area.width, main_h),
                Some(Rect::new(
                    right_area.x,
                    right_area.y + main_h,
                    right_area.width,
                    output_h,
                )),
            )
        } else {
            (right_area, None)
        };

        let show_matches = self.show_result.as_ref().is_some_and(|sr| {
            let sel = match self.active_tab {
                TAB_PACKAGES => self.packages.selected_package().map(|p| p.name.as_str()),
                _ => None,
            };
            sel.is_some_and(|name| sr.name == name)
        });

        // When output is visible, skip tags/framework bottom panel to save space
        let needs_bottom_panel =
            !self.output.is_visible() && (show_matches || self.framework_info.is_some());
        let (detail_area, bottom_area) = if needs_bottom_panel {
            let bottom_h = if show_matches {
                // Tags panel: larger
                (main_right_area.height / 3)
                    .max(8)
                    .min(main_right_area.height)
            } else {
                // Framework panel: compact
                7u16.min(main_right_area.height / 3)
            };
            let detail_h = main_right_area.height.saturating_sub(bottom_h);
            (
                Rect::new(
                    main_right_area.x,
                    main_right_area.y,
                    main_right_area.width,
                    detail_h,
                ),
                Some(Rect::new(
                    main_right_area.x,
                    main_right_area.y + detail_h,
                    main_right_area.width,
                    bottom_h,
                )),
            )
        } else {
            (main_right_area, None)
        };

        if show_matches {
            let pkg = self.packages.selected_package();
            let outdated_info =
                pkg.and_then(|p| self.outdated_result.iter().find(|o| o.name == p.name));
            render_show_detail(
                self.show_result.as_ref().unwrap(),
                pkg,
                outdated_info,
                detail_area,
                f.buffer_mut(),
                self.detail_focus,
                &mut self.detail_scroll,
            );
        } else {
            match self.active_tab {
                TAB_PACKAGES => {
                    let outdated_info = self
                        .packages
                        .selected_package()
                        .and_then(|pkg| self.outdated_result.iter().find(|o| o.name == pkg.name));
                    panels::detail::render_detail(
                        self.packages.selected_package(),
                        outdated_info,
                        detail_area,
                        f.buffer_mut(),
                        false,
                    );
                }
                TAB_AUDIT => {
                    panels::audit::render_audit_detail(
                        self.audit.selected_entry(),
                        detail_area,
                        f.buffer_mut(),
                        self.detail_focus,
                        &mut self.detail_scroll,
                    );
                }
                _ => {}
            }
        }

        // Bottom-right panel: tags (when show active) or framework
        if let Some(area) = bottom_area {
            if show_matches {
                render_tags_panel(self.show_result.as_ref().unwrap(), area, f.buffer_mut());
            } else if let Some(fw) = &self.framework_info {
                panels::packages::render_framework_panel(fw, area, f.buffer_mut());
            }
        }

        // Output panel: persistent bottom-right split
        if let Some(area) = output_bottom_area {
            self.output.set_size(area.width, area.height);
            self.output.render(area, f.buffer_mut());
        }
    }

    fn render_status_bar(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let bg = Style::default()
            .fg(theme::COLOR_TEXT)
            .bg(ratatui::style::Color::Rgb(0x1A, 0x1A, 0x1A));
        // Fill background
        for x in area.x..area.x + area.width {
            buf.set_string(x, area.y, " ", bg);
        }

        let mut x = area.x + 1;

        // Spinner
        if self.spinner.is_active() {
            let spinner_text = self.spinner.view();
            buf.set_string(x, area.y, &spinner_text, styles::version_style());
            x += spinner_text.len() as u16 + 2;
        }

        // Loading message
        let loading_msg = self.loading_message();
        if !loading_msg.is_empty() {
            let loading_style = Style::default()
                .fg(ratatui::style::Color::Rgb(0x00, 0x00, 0x00))
                .bg(theme::COLOR_WARNING)
                .add_modifier(ratatui::style::Modifier::BOLD);
            let msg = format!("⟳ {loading_msg}");
            buf.set_string(x, area.y, &msg, loading_style);
            x += msg.len() as u16 + 2;
        }

        // Error message
        if let Some(ref err) = self.err {
            buf.set_string(x, area.y, err, styles::error_style());
            x += err.len() as u16 + 2;
        }

        // Hints
        let hints = if self.output.is_visible() {
            vec![Hint {
                key: "q/esc".to_string(),
                desc: "back".to_string(),
            }]
        } else if self.help.is_visible() {
            vec![Hint {
                key: "esc/?".to_string(),
                desc: "close".to_string(),
            }]
        } else {
            default_hints(self.active_tab)
        };
        for hint in &hints {
            buf.set_string(x, area.y, &hint.key, styles::key_style());
            x += hint.key.len() as u16 + 1;
            buf.set_string(x, area.y, &hint.desc, bg);
            x += hint.desc.len() as u16 + 2;
        }

        // Right side: composer info
        if !self.composer_info.is_empty() {
            let info_len = self.composer_info.len() as u16;
            let right_x = area.x + area.width.saturating_sub(info_len + 2);
            if right_x > x {
                buf.set_string(right_x, area.y, &self.composer_info, styles::muted_style());
            }
        }
    }

    fn loading_message(&self) -> String {
        if self.loading_packages {
            return "Loading packages…".to_string();
        }
        let mut parts = Vec::new();
        if self.loading_outdated {
            parts.push("outdated");
        }
        if self.loading_audit {
            parts.push("audit");
        }
        if !parts.is_empty() {
            return format!("Analyzing {}…", parts.join(", "));
        }
        String::new()
    }

    // --- Async loading ---

    fn spawn_load_packages(&mut self) {
        self.loading_packages = true;
        self.spinner.start("Loading…");
        let dir = self.dir.clone();
        let tx = self.bg_tx.clone();
        thread::spawn(move || {
            let parser = composer::Parser::new();
            let cj = match parser.parse_json(&dir) {
                Ok(cj) => cj,
                Err(e) => {
                    let _ = tx.send(BgMsg::PackagesError(e));
                    return;
                }
            };
            let cl = match parser.parse_lock(&dir) {
                Ok(cl) => cl,
                Err(e) => {
                    let _ = tx.send(BgMsg::PackagesError(e));
                    return;
                }
            };
            let packages = parser.merge_packages(&cj, &cl);
            let framework = cj.extra.as_ref().and_then(composer::detect_framework);

            let lock_path = std::path::Path::new(&dir).join("composer.lock");
            let lock_hash = match std::fs::read(&lock_path) {
                Ok(data) => {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    data.hash(&mut hasher);
                    format!("{:x}", hasher.finish())
                }
                Err(_) => String::new(),
            };

            let _ = tx.send(BgMsg::PackagesLoaded {
                packages,
                lock_hash,
                framework,
                composer_json: Box::new(cj),
            });
        });
    }

    fn spawn_load_outdated(&mut self) {
        self.loading_outdated = true;
        self.spinner.start("Analyzing…");
        let dir = self.dir.clone();
        let runner = self.runner.clone();
        let tx = self.bg_tx.clone();
        thread::spawn(move || match runner.outdated(&dir) {
            Ok(result) => {
                let _ = tx.send(BgMsg::OutdatedLoaded(result));
            }
            Err(e) => {
                let _ = tx.send(BgMsg::OutdatedError(e));
            }
        });
    }

    fn spawn_load_audit(&mut self) {
        self.loading_audit = true;
        let dir = self.dir.clone();
        let runner = self.runner.clone();
        let tx = self.bg_tx.clone();
        thread::spawn(move || match runner.audit(&dir) {
            Ok(result) => {
                let _ = tx.send(BgMsg::AuditLoaded(result));
            }
            Err(e) => {
                let _ = tx.send(BgMsg::AuditError(e));
            }
        });
    }

    fn handle_show_selected(&mut self) {
        let pkg_name = match self.active_tab {
            TAB_PACKAGES => self.packages.selected_package().map(|p| p.name.clone()),
            _ => None,
        };
        if let Some(name) = pkg_name {
            // If show already loaded for this package, focus the detail panel
            if self.show_result.as_ref().is_some_and(|sr| sr.name == name) {
                self.detail_focus = true;
                self.detail_scroll = 0;
            } else {
                self.spawn_load_show(&name);
            }
        }
    }

    fn spawn_load_show(&mut self, pkg: &str) {
        self.loading_show = true;
        self.show_result = None;
        self.spinner.start("Loading show…");
        let dir = self.dir.clone();
        let runner = self.runner.clone();
        let tx = self.bg_tx.clone();
        let pkg = pkg.to_string();
        thread::spawn(move || match runner.show(&dir, &pkg) {
            Ok(result) => {
                let _ = tx.send(BgMsg::ShowLoaded(Box::new(result)));
            }
            Err(e) => {
                let _ = tx.send(BgMsg::ShowError(e));
            }
        });
    }

    fn spawn_resolve_all_restricted(&mut self) {
        let constraint = match &self.framework_info {
            Some(composer::FrameworkInfo::Symfony(sf)) if !sf.require.is_empty() => {
                sf.require.clone()
            }
            _ => return,
        };
        let restricted_pkgs: Vec<String> = self
            .packages
            .packages
            .iter()
            .filter(|p| p.status == composer::PackageStatus::Restricted)
            .map(|p| p.name.clone())
            .collect();
        for name in restricted_pkgs {
            self.spawn_load_restricted_version(&name, &constraint);
        }
    }

    fn spawn_load_restricted_version(&mut self, pkg_name: &str, constraint: &str) {
        if self.resolving_restricted.contains(pkg_name) {
            return;
        }
        self.resolving_restricted.insert(pkg_name.to_string());
        let dir = self.dir.clone();
        let runner = self.runner.clone();
        let tx = self.bg_tx.clone();
        let name = pkg_name.to_string();
        let constraint = constraint.to_string();
        thread::spawn(move || {
            let best_version = runner.show_all(&dir, &name).ok().and_then(|show| {
                composer::find_best_version_in_constraint(&show.versions, &constraint)
            });
            let _ = tx.send(BgMsg::RestrictedVersionLoaded { name, best_version });
        });
    }

    fn spawn_load_why_for_transitive(&mut self) {
        let transitive_pkgs = self.audit.transitive_advisory_packages();
        for pkg in transitive_pkgs {
            if self.resolving_why.contains(&pkg) {
                continue;
            }
            self.resolving_why.insert(pkg.clone());
            let runner = self.runner.clone();
            let tx = self.bg_tx.clone();
            let dir = self.dir.clone();
            thread::spawn(move || {
                let entries = runner.why(&dir, &pkg).unwrap_or_default();
                let _ = tx.send(BgMsg::WhyLoaded { pkg, entries });
            });
        }
    }

    fn spawn_load_composer_info(&self) {
        let runner = self.runner.clone();
        let tx = self.bg_tx.clone();
        thread::spawn(move || {
            let version = runner.version();
            let path = runner.bin_path();
            let _ = tx.send(BgMsg::ComposerInfo { version, path });
        });
    }

    fn handle_require(&mut self, pkg: &str) {
        match security::validate_package_name(pkg) {
            Err(e) => {
                self.output.show("composer require");
                self.output.append_line(&format!("Invalid input: {e}"));
            }
            Ok(validated) => {
                self.exec_require(&validated);
            }
        }
    }

    fn handle_update_selected(&mut self) {
        let (pkg_name, current_version, latest_version, latest_status, is_dev) =
            match self.active_tab {
                TAB_PACKAGES => {
                    if let Some(pkg) = self.packages.selected_package() {
                        let name = pkg.name.clone();
                        let version = pkg.version.clone();
                        let dev = pkg.is_dev;
                        // Look up outdated info
                        let (latest, status) = self
                            .outdated_result
                            .iter()
                            .find(|o| o.name == name)
                            .map(|o| (o.latest.clone(), o.latest_status.clone()))
                            .unwrap_or_default();
                        (Some(name), version, latest, status, dev)
                    } else {
                        return;
                    }
                }
                _ => return,
            };

        let name = match pkg_name {
            Some(n) => n,
            None => return,
        };

        // For major version updates, show a choice dialog with two options
        if latest_status == "update-possible"
            && !current_version.is_empty()
            && !latest_version.is_empty()
        {
            // Check if framework constraint blocks the upgrade
            let blocked_by_framework = self.is_upgrade_blocked_by_framework(&name, &latest_version);

            if blocked_by_framework {
                // Framework constraint blocks the major upgrade — only allow update within constraints
                let fw_label = self.framework_constraint_label();
                let msg = format!(
                    "composer update {}  ({}) — {} blocks upgrade to {}",
                    name, current_version, fw_label, latest_version
                );
                self.pending_action = Some(PendingAction::Update(name));
                self.confirm.show(&msg);
                return;
            }

            let major_constraint = format!("^{}", major_version(&latest_version));
            let msg = format!("composer update {}  ({})", name, current_version);
            self.choice.show(
                &msg,
                vec![
                    Choice {
                        key: 'u',
                        label: "Update within current constraints".to_string(),
                    },
                    Choice {
                        key: 'U',
                        label: format!("Upgrade to {} ({})", latest_version, major_constraint),
                    },
                ],
            );
            // Store both actions: Update for 'u', Upgrade for 'U'
            // We use a special dual pending action approach:
            // The choice handler will match 'u' -> Update, 'U' -> Upgrade
            self.pending_action = Some(PendingAction::Update(name.clone()));
            // Store upgrade target separately
            self.pending_upgrade_target = Some(format!("{}:{}", name, major_constraint));
            self.pending_upgrade_dev = is_dev;
            return;
        }

        // For semver-safe updates, show a confirm with versions
        let confirm_msg = if latest_status == "semver-safe-update"
            && !current_version.is_empty()
            && !latest_version.is_empty()
        {
            format!(
                "`composer update {}`  {} \u{2192} {}?",
                name, current_version, latest_version
            )
        } else if !current_version.is_empty() {
            format!("`composer update {}`  {}?", name, current_version)
        } else {
            format!("Run `composer update {}`?", name)
        };

        self.pending_action = Some(PendingAction::Update(name));
        self.confirm.show(&confirm_msg);
    }

    /// Checks if a major upgrade for this package is blocked by the framework constraint.
    fn is_upgrade_blocked_by_framework(&self, pkg_name: &str, latest_version: &str) -> bool {
        if let Some(composer::FrameworkInfo::Symfony(ref sf)) = self.framework_info {
            if composer::is_symfony_package(pkg_name) && !sf.require.is_empty() {
                return !composer::version_within_framework(latest_version, &sf.require);
            }
        }
        false
    }

    /// Returns a human-readable label for the framework constraint.
    fn framework_constraint_label(&self) -> String {
        match &self.framework_info {
            Some(composer::FrameworkInfo::Symfony(sf)) => {
                format!("Symfony {}", sf.require)
            }
            None => String::new(),
        }
    }

    fn exec_remove(&mut self, name: &str) {
        if self.stream_rx.is_some() {
            return;
        }
        let title = format!("composer remove {name}");
        self.output.show(&title);
        match self.runner.remove(&self.dir, name) {
            Ok(handle) => {
                self.stream_rx = Some(handle.rx);
                self.child_pid = handle.child_pid;
                self.loading = true;
            }
            Err(e) => {
                self.output.append_line(&format!("Error: {e}"));
            }
        }
    }

    fn exec_require(&mut self, pkg: &str) {
        if self.stream_rx.is_some() {
            return;
        }
        let title = format!("composer require {pkg}");
        self.output.show(&title);
        match self.runner.require(&self.dir, pkg) {
            Ok(handle) => {
                self.stream_rx = Some(handle.rx);
                self.child_pid = handle.child_pid;
                self.loading = true;
            }
            Err(e) => {
                self.output.append_line(&format!("Error: {e}"));
            }
        }
    }

    fn exec_upgrade(&mut self, pkg: &str, dev: bool) {
        if self.stream_rx.is_some() {
            return;
        }
        let dev_flag = if dev { " --dev" } else { "" };
        let title = format!("composer require --update-with-all-dependencies{dev_flag} {pkg}");
        self.output.show(&title);
        match self.runner.upgrade(&self.dir, pkg, dev) {
            Ok(handle) => {
                self.stream_rx = Some(handle.rx);
                self.child_pid = handle.child_pid;
                self.loading = true;
            }
            Err(e) => {
                self.output.append_line(&format!("Error: {e}"));
            }
        }
    }

    fn exec_update(&mut self, name: &str) {
        if self.stream_rx.is_some() {
            return;
        }
        let title = if name.is_empty() {
            "composer update".to_string()
        } else {
            format!("composer update {name}")
        };
        self.output.show(&title);
        match self.runner.update(&self.dir, name) {
            Ok(handle) => {
                self.stream_rx = Some(handle.rx);
                self.child_pid = handle.child_pid;
                self.loading = true;
            }
            Err(e) => {
                self.output.append_line(&format!("Error: {e}"));
            }
        }
    }
}

/// Extracts the major version number from a version string.
/// "v8.0.3" -> "8.0", "8.1.2" -> "8.0", "v2.0.0-beta1" -> "2.0"
fn major_version(version: &str) -> String {
    let v = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<&str> = v.splitn(3, '.').collect();
    if parts.is_empty() {
        return v.to_string();
    }
    format!("{}.0", parts[0])
}

/// Renders enriched detail panel from `composer show` result.
fn render_show_detail(
    show: &composer::ShowResult,
    pkg: Option<&composer::Package>,
    outdated: Option<&composer::OutdatedPackage>,
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    focused: bool,
    scroll: &mut u16,
) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Widget;

    let border_color = if focused {
        theme::COLOR_BORDER_FOCUS
    } else {
        theme::COLOR_BORDER
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(" show ", styles::title_style()));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();

    // Name
    lines.push(Line::from(Span::styled(&show.name, styles::title_style())));
    lines.push(Line::default());

    // Version
    if let Some(v) = show.versions.first() {
        lines.push(show_field("Version:", v, styles::version_style()));
    }

    // Constraint (from Package)
    if let Some(p) = pkg {
        if !p.constraint.is_empty() {
            let bounds = composer::explain_constraint(&p.constraint);
            lines.push(Line::from(vec![
                Span::styled("Constraint:", styles::key_style()),
                Span::raw("  "),
                Span::styled(&p.constraint, styles::version_style()),
                Span::raw("  "),
                Span::styled(format!("({bounds})"), styles::muted_style()),
            ]));
        }
    }

    // Outdated info (from outdated result)
    if let Some(p) = pkg {
        if let Some(o) = outdated {
            if let Some(rv) = &p.restricted_latest {
                if p.status != composer::PackageStatus::OK {
                    lines.push(Line::from(vec![
                        Span::styled("Latest:", styles::key_style()),
                        Span::raw("  "),
                        Span::styled(rv, styles::package_outdated_style()),
                        Span::styled(" (within framework)", styles::muted_style()),
                    ]));
                }
            } else if p.status == composer::PackageStatus::Restricted {
                lines.push(Line::from(vec![
                    Span::styled("Latest:", styles::key_style()),
                    Span::raw("  "),
                    Span::styled(&o.latest, styles::muted_style()),
                    Span::styled(" (blocked by framework)", styles::muted_style()),
                ]));
            } else {
                let status_color = theme::status_color(&o.latest_status);
                let version_style = Style::default().fg(status_color);
                lines.push(Line::from(vec![
                    Span::styled("Latest:", styles::key_style()),
                    Span::raw("  "),
                    Span::styled(&o.latest, version_style),
                ]));
            }
        }

        // Status
        let status_span = match p.status {
            composer::PackageStatus::Vulnerable => {
                Span::styled("Vulnerable", styles::package_vulnerable_style())
            }
            composer::PackageStatus::Abandoned => {
                Span::styled("Abandoned", styles::package_abandoned_style())
            }
            composer::PackageStatus::Outdated => {
                Span::styled("Outdated", styles::package_outdated_style())
            }
            composer::PackageStatus::Restricted => {
                Span::styled("Restricted", styles::package_restricted_style())
            }
            _ => Span::styled("OK", styles::package_ok_style()),
        };
        lines.push(Line::from(vec![
            Span::styled("Status:", styles::key_style()),
            Span::raw("  "),
            status_span,
        ]));
    }

    // Description
    if !show.description.is_empty() {
        lines.extend(crate::ui::text::wrap_field(
            "Description:",
            &show.description,
            Style::default().fg(theme::COLOR_TEXT),
            inner.width,
        ));
    }

    // Type
    if !show.pkg_type.is_empty() {
        lines.push(show_field(
            "Type:",
            &show.pkg_type,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }

    // License
    let license_joined;
    if !show.licenses.is_empty() {
        let license_str: Vec<&str> = show.licenses.iter().map(|l| l.name.as_str()).collect();
        license_joined = license_str.join(", ");
        lines.push(show_field(
            "License:",
            &license_joined,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }

    // Homepage
    if !show.homepage.is_empty() {
        lines.push(show_field(
            "Homepage:",
            &show.homepage,
            Style::default().fg(theme::COLOR_INFO),
        ));
    }

    // Source
    if !show.source.url.is_empty() {
        lines.push(show_field(
            "Source:",
            &show.source.url,
            Style::default().fg(theme::COLOR_INFO),
        ));
    }

    // Released
    if !show.released.is_empty() {
        let date = show.released.split('T').next().unwrap_or(&show.released);
        lines.push(show_field(
            "Released:",
            date,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }

    // Path
    if !show.path.is_empty() {
        lines.push(show_field("Path:", &show.path, styles::muted_style()));
    }

    // Keywords
    let keywords_joined;
    if !show.keywords.is_empty() {
        keywords_joined = show.keywords.join(", ");
        lines.push(show_field(
            "Keywords:",
            &keywords_joined,
            styles::muted_style(),
        ));
    }

    // Requires
    if !show.requires.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Requires ({})", show.requires.len()),
            styles::key_style(),
        )));
        let mut deps: Vec<_> = show.requires.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, constraint) in &deps {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
    }

    // Dev requires
    if !show.dev_requires.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Dev Requires ({})", show.dev_requires.len()),
            styles::dev_style(),
        )));
        let mut deps: Vec<_> = show.dev_requires.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, constraint) in deps.iter().take(15) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
        if show.dev_requires.len() > 15 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", show.dev_requires.len() - 15),
                styles::muted_style(),
            )));
        }
    }

    // Conflicts
    if !show.conflicts.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Conflicts ({})", show.conflicts.len()),
            Style::default().fg(theme::COLOR_WARNING),
        )));
        let mut deps: Vec<_> = show.conflicts.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, constraint) in deps.iter().take(10) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_WARNING)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
        if show.conflicts.len() > 10 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", show.conflicts.len() - 10),
                styles::muted_style(),
            )));
        }
    }

    // Replaces
    if !show.replaces.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Replaces ({})", show.replaces.len()),
            styles::key_style(),
        )));
        let mut deps: Vec<_> = show.replaces.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, constraint) in deps.iter().take(10) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
        if show.replaces.len() > 10 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", show.replaces.len() - 10),
                styles::muted_style(),
            )));
        }
    }

    // Provides
    if !show.provides.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Provides ({})", show.provides.len()),
            styles::key_style(),
        )));
        let mut deps: Vec<_> = show.provides.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, constraint) in deps.iter().take(10) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
        if show.provides.len() > 10 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", show.provides.len() - 10),
                styles::muted_style(),
            )));
        }
    }

    // Suggests
    if !show.suggests.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("Suggests ({})", show.suggests.len()),
            styles::muted_style(),
        )));
        let mut deps: Vec<_> = show.suggests.iter().collect();
        deps.sort_by_key(|(k, _)| k.as_str());
        for (name, desc) in deps.iter().take(10) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(desc.as_str(), styles::muted_style()),
            ]));
        }
        if show.suggests.len() > 10 {
            lines.push(Line::from(Span::styled(
                format!("  … and {} more", show.suggests.len() - 10),
                styles::muted_style(),
            )));
        }
    }

    // Clamp scroll to max
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_lines);
    *scroll = (*scroll).min(max_scroll);

    let paragraph = ratatui::widgets::Paragraph::new(lines).scroll((*scroll, 0));
    paragraph.render(inner, buf);
}

fn render_tags_panel(show: &composer::ShowResult, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Widget;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::COLOR_BORDER))
        .title(Span::styled(
            format!(" Tags ({}) ", show.versions.len()),
            styles::title_style(),
        ));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();
    for v in &show.versions {
        lines.push(Line::from(Span::styled(v, styles::version_style())));
    }

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

fn show_field<'a>(label: &'a str, value: &'a str, value_style: Style) -> ratatui::text::Line<'a> {
    use ratatui::text::{Line, Span};
    Line::from(vec![
        Span::styled(label, styles::key_style()),
        Span::raw("  "),
        Span::styled(value, value_style),
    ])
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = (r.width.saturating_sub(popup_width)) / 2 + r.x;
    let y = (r.height.saturating_sub(height)) / 2 + r.y;
    Rect::new(x, y, popup_width, height)
}
