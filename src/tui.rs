use crate::constants;
use crate::environment::{self, CheckDetail, CheckItem, CheckResult, UserInfo};
use crate::error::{Invalid, Result, ThumedError};
use crate::i18n::{check_name, error_text, fill, setup_step_name, Lang};
use crate::pod_handler::{wait_for_pod_running, PodConfig, PodHandler};
use crate::tools::SetupStep;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, Read, Stdout};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const MENU_COUNT: usize = 6;
const MAX_PODS_SHOWN: usize = 12;

#[derive(Clone, Copy)]
enum PodAction {
    Login,
    Forward,
    Uninstall,
}

impl PodAction {
    fn title(self, lang: Lang) -> &'static str {
        let t = lang.t();
        match self {
            Self::Login => t.title_login,
            Self::Forward => t.title_forward,
            Self::Uninstall => t.title_uninstall,
        }
    }
}

#[derive(Clone, Copy)]
enum FormKind {
    Install,
    Credentials,
}

impl FormKind {
    fn labels(self, lang: Lang) -> &'static [&'static str] {
        match self {
            Self::Install => &lang.t().install_labels,
            Self::Credentials => &lang.t().credential_labels,
        }
    }
    fn title(self, lang: Lang) -> &'static str {
        match self {
            Self::Install => lang.t().title_install,
            Self::Credentials => lang.t().title_credentials,
        }
    }
}

enum Screen {
    Initialize,
    Menu,
    Report(Vec<CheckResult>),
    InstallFailed { release: String, error: ThumedError },
    PodPicker(PodAction),
    Form(FormKind),
    ConfirmUninstall { pod_name: String, release: String },
    Forwarding { pod_name: String, child: Child },
}

/// Status messages without arguments are stored as a key so the language
/// toggle can re-render them; formatted ones keep their original wording.
#[derive(Clone, Copy)]
enum StatusKey {
    None,
    Loading,
    EnvDone,
    EnvIncomplete,
    WaitingForPod,
    NoPods,
    LoginEnded,
    ForwardStarted,
    ForwardStopped,
    Installing,
    Installed,
    UserSaved,
    UninstallCancelled,
    Checking,
}

impl StatusKey {
    fn text(self, lang: Lang) -> Option<&'static str> {
        let t = lang.t();
        Some(match self {
            Self::None => return None,
            Self::Loading => t.status_loading,
            Self::EnvDone => t.status_env_done,
            Self::EnvIncomplete => t.status_env_incomplete,
            Self::WaitingForPod => t.pod_waiting,
            Self::NoPods => t.status_no_pods,
            Self::LoginEnded => t.status_login_ended,
            Self::ForwardStarted => t.status_forward_started,
            Self::ForwardStopped => t.status_forward_stopped,
            Self::Installing => t.status_installing,
            Self::Installed => t.status_installed,
            Self::UserSaved => t.status_user_saved,
            Self::UninstallCancelled => t.status_uninstall_cancelled,
            Self::Checking => t.check_running,
        })
    }
}

struct App {
    config_dir: PathBuf,
    selected: usize,
    pod_selected: usize,
    pod_number: String,
    form_selected: usize,
    form_values: Vec<String>,
    credentials: Option<UserInfo>,
    report_scroll: u16,
    screen: Screen,
    status: String,
    status_key: StatusKey,
    lang: Lang,
}

impl App {
    fn new(config_dir: PathBuf) -> Self {
        let lang = Lang::Zh;
        let initialized = environment::is_initialized(&config_dir);
        let status_key = if initialized {
            StatusKey::Loading
        } else {
            StatusKey::None
        };
        Self {
            config_dir,
            selected: 0,
            pod_selected: 0,
            pod_number: String::new(),
            form_selected: 0,
            form_values: Vec::new(),
            credentials: None,
            report_scroll: 0,
            screen: if initialized {
                Screen::Menu
            } else {
                Screen::Initialize
            },
            status: status_key.text(lang).unwrap_or_default().to_string(),
            status_key,
            lang,
        }
    }

    fn set_status(&mut self, key: StatusKey) {
        self.status_key = key;
        self.status = key.text(self.lang).unwrap_or_default().to_string();
    }

    fn set_status_text(&mut self, text: String) {
        self.status_key = StatusKey::None;
        self.status = text;
    }

    fn set_error(&mut self, error: &ThumedError) {
        self.set_status_text(error_text(error, self.lang));
    }

    fn toggle_lang(&mut self) {
        self.lang = self.lang.toggle();
        if let Some(text) = self.status_key.text(self.lang) {
            self.status = text.to_string();
        }
    }

    fn move_selection(&mut self, delta: isize) {
        self.selected = move_index(self.selected, delta, MENU_COUNT);
    }

    fn begin_form(&mut self) {
        self.form_selected = 0;
        self.credentials = None;
        self.form_values = vec![String::new(); self.lang.t().install_labels.len()];
        self.screen = Screen::Form(FormKind::Install);
    }

    fn begin_credentials(&mut self) -> Result<()> {
        // A damaged override can still be repaired using kubeconfig defaults.
        let info = UserInfo::load().or_else(|_| UserInfo::from_kubeconfig())?;
        self.form_selected = 0;
        self.form_values = vec![info.user.clone(), info.password.clone()];
        self.credentials = Some(info);
        self.screen = Screen::Form(FormKind::Credentials);
        Ok(())
    }
}

fn move_index(index: usize, delta: isize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    (index as isize + delta).rem_euclid(count as isize) as usize
}

fn append_pod_number(number: &mut String, digit: char) -> bool {
    if !digit.is_ascii_digit() || (digit == '0' && number.is_empty()) {
        return false;
    }
    number.push(digit);
    true
}

type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run(config_dir: &Path, pod_handler: &mut PodHandler) -> Result<()> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let mut app = App::new(config_dir.to_path_buf());
    if !environment::kubeconfig_path().is_file() {
        app.screen = Screen::Initialize;
    }

    terminal.draw(|frame| draw_ui(frame, &app, pod_handler))?;
    if matches!(app.screen, Screen::Menu) {
        refresh_pods(&mut app, pod_handler);
    }

    let result = loop {
        if matches!(app.screen, Screen::Initialize) && environment::kubeconfig_path().is_file() {
            // Once config is supplied, setup continues without another prompt.
            check_environment(&mut app, &mut terminal, pod_handler)?;
        }
        poll_forward(&mut app);
        terminal.draw(|frame| draw_ui(frame, &app, pod_handler))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::F(2) {
            app.toggle_lang();
            continue;
        }
        if matches!(app.screen, Screen::Menu)
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            break Ok(());
        }
        if let Err(error) = handle_key(&mut app, &mut terminal, pod_handler, key.code) {
            app.set_error(&error);
        }
    };

    stop_forward(&mut app);
    result
}

/// Notice a port-forward that died on its own and surface its stderr.
fn poll_forward(app: &mut App) {
    let Screen::Forwarding { child, .. } = &mut app.screen else {
        return;
    };
    if !matches!(child.try_wait(), Ok(Some(_))) {
        return;
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let message = fill(
        app.lang.t().status_forward_failed,
        &[stderr.trim().lines().last().unwrap_or("")],
    );
    app.set_status_text(message);
    app.screen = Screen::Menu;
}

fn stop_forward(app: &mut App) {
    if let Screen::Forwarding { child, .. } = &mut app.screen {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn handle_key(
    app: &mut App,
    terminal: &mut AppTerminal,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    match app.screen {
        Screen::Initialize => {
            match key {
                KeyCode::Enter => check_environment(app, terminal, pod_handler)?,
                KeyCode::Esc => {
                    app.selected = MENU_COUNT - 1;
                    app.screen = Screen::Menu;
                }
                _ => {}
            }
            Ok(())
        }
        Screen::Menu => handle_menu_key(app, terminal, pod_handler, key),
        Screen::Report(ref report) => {
            match key {
                KeyCode::Esc | KeyCode::Enter => app.screen = Screen::Menu,
                KeyCode::Char('r') => check_environment(app, terminal, pod_handler)?,
                KeyCode::Char('u') => app.begin_credentials()?,
                KeyCode::Up
                | KeyCode::Down
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End => {
                    let area = terminal.get_frame().area();
                    let max_scroll = report_paragraph(app, report, None)
                        .line_count(area.width.saturating_sub(2))
                        .saturating_sub(area.height.saturating_sub(5) as usize)
                        .min(u16::MAX as usize) as u16;
                    app.report_scroll = scroll_report(app.report_scroll, key, max_scroll);
                }
                _ => {}
            }
            Ok(())
        }
        Screen::InstallFailed { .. } => {
            if matches!(key, KeyCode::Enter | KeyCode::Esc) {
                app.screen = Screen::Menu;
            }
            Ok(())
        }
        Screen::PodPicker(action) => handle_pod_picker_key(app, terminal, pod_handler, action, key),
        Screen::Form(kind) => handle_form_key(app, terminal, pod_handler, kind, key),
        Screen::ConfirmUninstall { .. } => handle_uninstall_confirmation(app, pod_handler, key),
        Screen::Forwarding { .. } => {
            if matches!(key, KeyCode::Esc | KeyCode::Char('q')) {
                stop_forward(app);
                app.screen = Screen::Menu;
                app.set_status(StatusKey::ForwardStopped);
            }
            Ok(())
        }
    }
}

fn handle_menu_key(
    app: &mut App,
    terminal: &mut AppTerminal,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Enter => match app.selected {
            0 => app.begin_form(),
            1 => open_pod_picker(app, pod_handler, PodAction::Login),
            2 => open_pod_picker(app, pod_handler, PodAction::Forward),
            3 => open_pod_picker(app, pod_handler, PodAction::Uninstall),
            4 => app.begin_credentials()?,
            5 => check_environment(app, terminal, pod_handler)?,
            _ => unreachable!(),
        },
        _ => {}
    }
    Ok(())
}

fn check_environment(
    app: &mut App,
    terminal: &mut AppTerminal,
    _pod_handler: &PodHandler,
) -> Result<()> {
    app.set_status(StatusKey::Checking);
    app.report_scroll = 0;
    let report = environment::check_env(|report, item, step| {
        terminal.draw(|frame| {
            let area = frame.area();
            draw_report(frame, area, app, report, Some((item, step)));
        })?;
        Ok(())
    })?;
    let initialized = environment::record_initialization(&app.config_dir, &report);
    // Keep successful reports visible too, until the user explicitly leaves.
    app.screen = Screen::Report(report);
    match initialized {
        Ok(true) => {
            app.selected = 0;
            app.set_status(StatusKey::EnvDone);
        }
        Ok(false) => {
            app.selected = MENU_COUNT - 1;
            app.set_status(StatusKey::EnvIncomplete);
        }
        Err(error) => {
            app.selected = MENU_COUNT - 1;
            app.set_error(&error);
        }
    }
    // Do not let keys queued while installing instantly dismiss the report.
    while event::poll(Duration::ZERO)? {
        let _ = event::read()?;
    }
    Ok(())
}

fn finish_installation(
    app: &mut App,
    pod_handler: &mut PodHandler,
    release: String,
    result: Result<Vec<String>>,
) {
    app.form_values.clear();
    match result {
        Ok(names) => {
            pod_handler.pod_list.extend(names);
            pod_handler.pod_list.sort();
            pod_handler.pod_list.dedup();
            app.selected = 0;
            app.screen = Screen::Menu;
            app.set_status(StatusKey::Installed);
        }
        Err(error) => {
            app.selected = if matches!(error, ThumedError::PodStartupFailed { .. }) {
                3
            } else {
                0
            };
            app.set_error(&error);
            app.screen = Screen::InstallFailed { release, error };
        }
    }
}

fn open_pod_picker(app: &mut App, pod_handler: &mut PodHandler, action: PodAction) {
    refresh_pods(app, pod_handler);
    if pod_handler.pod_list.is_empty() {
        app.set_status(StatusKey::NoPods);
    } else {
        app.pod_selected = 0;
        app.pod_number.clear();
        app.screen = Screen::PodPicker(action);
    }
}

fn refresh_pods(app: &mut App, pod_handler: &mut PodHandler) {
    match pod_handler.refresh() {
        Ok(()) => {
            let text = fill(
                app.lang.t().status_pods_loaded,
                &[&pod_handler.pod_list.len().to_string()],
            );
            app.set_status_text(text);
        }
        Err(error) => app.set_error(&error),
    }
}

fn handle_pod_picker_key(
    app: &mut App,
    terminal: &mut AppTerminal,
    pod_handler: &mut PodHandler,
    action: PodAction,
    key: KeyCode,
) -> Result<()> {
    let count = pod_handler.pod_list.len();
    match key {
        KeyCode::Esc => app.screen = Screen::Menu,
        KeyCode::Up | KeyCode::Char('k') => {
            app.pod_selected = move_index(app.pod_selected, -1, count)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.pod_selected = move_index(app.pod_selected, 1, count)
        }
        KeyCode::Backspace => {
            app.pod_number.pop();
        }
        KeyCode::Char(digit) if append_pod_number(&mut app.pod_number, digit) => {
            if let Ok(number) = app.pod_number.parse::<usize>() {
                if number >= 1 && number <= count {
                    app.pod_selected = number - 1;
                }
            }
        }
        KeyCode::Enter => {
            if !app.pod_number.is_empty() {
                let number = app.pod_number.parse::<usize>().unwrap_or(0);
                if number == 0 || number > count {
                    let text = fill(app.lang.t().status_pod_range, &[&count.to_string()]);
                    app.set_status_text(text);
                    return Ok(());
                }
                app.pod_selected = number - 1;
            }
            let pod_name = pod_handler.pod_list[app.pod_selected].clone();
            match action {
                PodAction::Login => {
                    app.screen = Screen::Menu;
                    with_terminal_suspended(terminal, || pod_handler.login_pod_by_name(&pod_name))?;
                    app.set_status(StatusKey::LoginEnded);
                }
                PodAction::Forward => {
                    let child = pod_handler.start_forward(&pod_name)?;
                    app.screen = Screen::Forwarding { pod_name, child };
                    app.set_status(StatusKey::ForwardStarted);
                }
                PodAction::Uninstall => {
                    let release = pod_handler.release_for_pod(&pod_name)?;
                    app.screen = Screen::ConfirmUninstall { pod_name, release };
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_form_key(
    app: &mut App,
    terminal: &mut AppTerminal,
    pod_handler: &mut PodHandler,
    kind: FormKind,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Esc => {
            app.credentials = None;
            app.form_values.clear();
            app.screen = Screen::Menu;
        }
        KeyCode::Up | KeyCode::BackTab => {
            app.form_selected = move_index(app.form_selected, -1, app.form_values.len())
        }
        KeyCode::Down | KeyCode::Tab => {
            app.form_selected = move_index(app.form_selected, 1, app.form_values.len())
        }
        KeyCode::Backspace => {
            app.form_values[app.form_selected].pop();
        }
        KeyCode::Delete => app.form_values[app.form_selected].clear(),
        KeyCode::Char(character) => app.form_values[app.form_selected].push(character),
        KeyCode::Enter => {
            if matches!(kind, FormKind::Credentials) {
                let info = app.credentials.as_mut().ok_or(Invalid::SavedCredentials)?;
                info.user = app.form_values[0].trim().to_string();
                info.password = app.form_values[1].clone();
                info.save(&app.config_dir)?;
                app.credentials = None;
                app.form_values.clear();
                app.screen = Screen::Menu;
                app.set_status(StatusKey::UserSaved);
            } else {
                submit_form(app, terminal, pod_handler)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn submit_form(
    app: &mut App,
    terminal: &mut AppTerminal,
    pod_handler: &mut PodHandler,
) -> Result<()> {
    let pod_config = PodConfig::from_values(
        &app.form_values[0],
        &app.form_values[1],
        &app.form_values[2],
    )?;
    app.set_status(StatusKey::Installing);
    terminal.draw(|frame| draw_ui(frame, app, pod_handler))?;
    let result = match pod_config.install_pod() {
        Ok(()) => {
            app.set_status(StatusKey::WaitingForPod);
            terminal.draw(|frame| draw_ui(frame, app, pod_handler))?;
            wait_for_pod_running(pod_config.release_name())
        }
        Err(error) => Err(error),
    };
    // Discard keys typed during the blocking operation, so they cannot
    // accidentally submit another form or dismiss a failure message.
    while event::poll(Duration::ZERO)? {
        let _ = event::read()?;
    }
    finish_installation(
        app,
        pod_handler,
        pod_config.release_name().to_string(),
        result,
    );
    Ok(())
}

fn handle_uninstall_confirmation(
    app: &mut App,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    let Screen::ConfirmUninstall { pod_name, release } = &app.screen else {
        return Ok(());
    };
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let (pod_name, release) = (pod_name.clone(), release.clone());
            pod_handler.uninstall_pod_release(&pod_name, &release)?;
            let text = fill(app.lang.t().status_uninstalled, &[&pod_name, &release]);
            app.set_status_text(text);
            app.screen = Screen::Menu;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.set_status(StatusKey::UninstallCancelled);
            app.screen = Screen::Menu;
        }
        _ => {}
    }
    Ok(())
}

fn draw_ui(frame: &mut Frame, app: &App, pod_handler: &PodHandler) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", constants::APP_NAME),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {}", app.lang.t().version)),
        Span::styled(
            format!("   [F2 {}]", app.lang.name()),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, layout[0]);

    match &app.screen {
        Screen::Initialize => draw_initialize(frame, layout[1], app),
        Screen::Menu => draw_menu(frame, layout[1], app, pod_handler),
        Screen::InstallFailed { release, error } => {
            draw_install_failure(frame, layout[1], app, release, error)
        }
        Screen::Report(report) => draw_report(frame, layout[1], app, report, None),
        Screen::PodPicker(action) => draw_pod_picker(frame, layout[1], app, pod_handler, *action),
        Screen::Form(kind) => draw_form(frame, layout[1], app, *kind),
        Screen::ConfirmUninstall { pod_name, release } => {
            draw_uninstall_confirmation(frame, layout[1], app, pod_name, release)
        }
        Screen::Forwarding { pod_name, .. } => draw_forwarding(frame, layout[1], app, pod_name),
    }

    let t = app.lang.t();
    let footer = match app.screen {
        Screen::Initialize => t.footer_initialize,
        Screen::InstallFailed { .. } => t.footer_install_result,
        Screen::Form(FormKind::Install)
            if matches!(
                app.status_key,
                StatusKey::Installing | StatusKey::WaitingForPod
            ) =>
        {
            t.footer_install_wait
        }
        Screen::Menu => t.footer_menu,
        Screen::Report(_) => t.footer_back,
        Screen::PodPicker(_) => t.footer_picker,
        Screen::Form(_) => t.footer_form,
        Screen::ConfirmUninstall { .. } => t.footer_confirm,
        Screen::Forwarding { .. } => t.footer_forward,
    };
    frame.render_widget(
        Paragraph::new(footer)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: true }),
        layout[2],
    );
}

fn draw_initialize(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.lang.t();
    let lines = if matches!(app.status_key, StatusKey::Checking) {
        vec![Line::from(t.check_running)]
    } else {
        vec![
            Line::from(t.initialize_hint),
            Line::from(""),
            Line::from(environment::kubeconfig_path().display().to_string()),
        ]
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(t.panel_initialize)
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_install_failure(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    release: &str,
    error: &ThumedError,
) {
    let t = app.lang.t();
    let message = error_text(error, app.lang);
    let mut lines = vec![
        Line::from(fill(t.uninstall_release, &[release])),
        Line::from(""),
    ];
    lines.extend(
        message
            .lines()
            .map(|line| Line::styled(line, Style::default().fg(Color::Red))),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(t.panel_install_failed)
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_menu(frame: &mut Frame, area: Rect, app: &App, pod_handler: &PodHandler) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let t = app.lang.t();
    let items = t
        .menu_labels
        .iter()
        .map(|label| ListItem::new(*label))
        .collect::<Vec<_>>();
    let menu = List::new(items)
        .block(
            Block::default()
                .title(t.panel_actions)
                .borders(Borders::ALL),
        )
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    let mut menu_state = ListState::default();
    menu_state.select(Some(app.selected));
    frame.render_stateful_widget(menu, columns[0], &mut menu_state);

    let mut details = vec![
        Line::from(Span::styled(
            t.menu_details[app.selected],
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            t.panel_pods,
            Style::default().fg(Color::Yellow),
        )),
    ];
    if pod_handler.pod_list.is_empty() {
        details.push(Line::from(t.no_pods_loaded));
    } else {
        for (index, pod) in pod_handler.pod_list.iter().take(MAX_PODS_SHOWN).enumerate() {
            details.push(Line::from(format!("{:>2}. {}", index + 1, pod)));
        }
        if pod_handler.pod_list.len() > MAX_PODS_SHOWN {
            details.push(Line::from(fill(
                t.and_more,
                &[&(pod_handler.pod_list.len() - MAX_PODS_SHOWN).to_string()],
            )));
        }
    }
    details.push(Line::from(""));
    details.push(status_line(app));
    frame.render_widget(
        Paragraph::new(details)
            .block(
                Block::default()
                    .title(t.panel_details)
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn scroll_report(current: u16, key: KeyCode, max: u16) -> u16 {
    let current = current.min(max);
    match key {
        KeyCode::Up => current.saturating_sub(1),
        KeyCode::Down => current.saturating_add(1).min(max),
        KeyCode::PageUp => current.saturating_sub(10),
        KeyCode::PageDown => current.saturating_add(10).min(max),
        KeyCode::Home => 0,
        KeyCode::End => max,
        _ => current,
    }
}

fn report_paragraph(
    app: &App,
    report: &[CheckResult],
    active: Option<(CheckItem, SetupStep)>,
) -> Paragraph<'static> {
    let t = app.lang.t();
    let mut lines = Vec::new();
    if let Some((item, step)) = active {
        let number = CheckItem::ALL.iter().position(|key| *key == item).unwrap() + 1;
        lines.push(Line::styled(
            format!(
                "[{}/{}] {} — {}",
                number,
                CheckItem::ALL.len(),
                check_name(item, app.lang),
                setup_step_name(step, app.lang)
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));
    }
    for item in CheckItem::ALL {
        let result = report
            .iter()
            .find(|(key, _)| *key == item)
            .map(|(_, result)| result);
        let (mark, color) = match result {
            Some(Ok(_)) => ("✔", Color::Green),
            Some(Err(ThumedError::Invalid(Invalid::SetupPrerequisite))) => ("-", Color::DarkGray),
            Some(Err(_)) => ("✘", Color::Red),
            None => ("…", Color::Yellow),
        };
        lines.push(Line::styled(
            format!("{} {}", mark, check_name(item, app.lang)),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        let details = match result {
            Some(Ok(CheckDetail::Text(detail))) => detail.clone(),
            Some(Ok(CheckDetail::Account(info))) => format!(
                "{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}",
                t.context_label,
                info.context,
                t.server_label,
                info.server,
                t.namespace_label,
                info.namespace,
                t.credential_labels[0],
                info.user,
                t.credential_labels[1],
                info.password,
                if info.customized {
                    t.account_customized
                } else {
                    t.account_default
                },
            ),
            Some(Err(error)) => error_text(error, app.lang),
            None => match active {
                Some((key, step)) if key == item => setup_step_name(step, app.lang).to_string(),
                _ => t.check_pending.to_string(),
            },
        };
        for line in details.lines() {
            lines.push(Line::from(format!("  {}", line)));
        }
        lines.push(Line::from(""));
    }
    if active.is_none() {
        lines.push(status_line(app));
    }
    Paragraph::new(lines)
        .block(Block::default().title(t.panel_report).borders(Borders::ALL))
        .wrap(Wrap { trim: false })
}

fn draw_report(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    report: &[CheckResult],
    active: Option<(CheckItem, SetupStep)>,
) {
    let paragraph = report_paragraph(app, report, active);
    let max = paragraph
        .line_count(area.width.saturating_sub(2))
        .saturating_sub(area.height as usize)
        .min(u16::MAX as usize) as u16;
    let scroll = if active.is_some() {
        0
    } else {
        app.report_scroll.min(max)
    };
    frame.render_widget(paragraph.scroll((scroll, 0)), area);
}

fn draw_pod_picker(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    pod_handler: &PodHandler,
    action: PodAction,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(area);
    let items = pod_handler
        .pod_list
        .iter()
        .enumerate()
        .map(|(index, pod)| ListItem::new(format!("{:>2}. {}", index + 1, pod)))
        .collect::<Vec<_>>();
    let title = if app.pod_number.is_empty() {
        action.title(app.lang).to_string()
    } else {
        format!(
            "{} — {}: {}",
            action.title(app.lang),
            app.lang.t().picker_number,
            app.pod_number
        )
    };
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(highlight())
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    state.select(Some(app.pod_selected));
    frame.render_stateful_widget(list, sections[0], &mut state);
    frame.render_widget(Paragraph::new(status_line(app)), sections[1]);
}

fn draw_form(frame: &mut Frame, area: Rect, app: &App, kind: FormKind) {
    let hint = if matches!(kind, FormKind::Install)
        && matches!(
            app.status_key,
            StatusKey::Installing | StatusKey::WaitingForPod
        ) {
        app.lang.t().footer_install_wait
    } else if matches!(kind, FormKind::Credentials) {
        app.lang.t().credentials_hint
    } else {
        app.lang.t().form_hint
    };
    let mut lines = vec![Line::from(hint), Line::from("")];
    if let Some(info) = &app.credentials {
        lines.push(Line::from(format!(
            "{}: {}",
            app.lang.t().context_label,
            info.context
        )));
        lines.push(Line::from(format!(
            "{}: {}",
            app.lang.t().server_label,
            info.server
        )));
        lines.push(Line::from(""));
    }
    for (index, label) in kind.labels(app.lang).iter().enumerate() {
        let value = &app.form_values[index];
        let selected = index == app.form_selected;
        lines.push(Line::from(Span::styled(
            format!("{} {}: {}", if selected { "▶" } else { " " }, label, value),
            if selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            },
        )));
    }
    lines.push(Line::from(""));
    lines.push(status_line(app));
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(kind.title(app.lang))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_forwarding(frame: &mut Frame, area: Rect, app: &App, pod_name: &str) {
    let t = app.lang.t();
    let port = constants::FORWARD_PORT.to_string();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                fill(t.forward_running, &[pod_name, &port]),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(fill(t.forward_url, &[&port])),
            Line::from(t.forward_stop),
            Line::from(""),
            status_line(app),
        ])
        .block(
            Block::default()
                .title(t.panel_forward)
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_uninstall_confirmation(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    pod_name: &str,
    release: &str,
) {
    let t = app.lang.t();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                t.uninstall_warning,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(fill(t.uninstall_pod, &[pod_name])),
            Line::from(fill(t.uninstall_release, &[release])),
            Line::from(""),
            Line::from(t.uninstall_continue),
            status_line(app),
        ])
        .block(
            Block::default()
                .title(t.panel_confirm)
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn highlight() -> Style {
    Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

fn status_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            app.lang.t().status_label,
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(app.status.clone()),
    ])
}

/// Leave the TUI so a child process can own the terminal (interactive shell).
fn with_terminal_suspended<T, F>(terminal: &mut AppTerminal, action: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let action_result = action();

    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    action_result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        App::new(std::env::temp_dir().join(format!("thumed_tui_test_{}", nanos)))
    }

    fn render(app: &App) -> String {
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 36)).unwrap();
        terminal
            .draw(|frame| draw_ui(frame, app, &PodHandler::new()))
            .unwrap();
        screen_text(&terminal)
    }

    fn screen_text(terminal: &Terminal<ratatui::backend::TestBackend>) -> String {
        let mut text = String::new();
        for row in terminal.backend().buffer().content.chunks(100) {
            let mut column = 0;
            while column < row.len() {
                let symbol = row[column].symbol();
                text.push_str(symbol);
                // Wide CJK glyphs occupy two cells; the second is only padding.
                column += Line::from(symbol).width().max(1);
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn first_launch_prompts_until_initialization_succeeds() {
        let app = app();
        let dir = app.config_dir.clone();
        assert!(matches!(app.screen, Screen::Initialize));
        assert_eq!(app.lang, Lang::Zh);
        let text = render(&app);
        assert!(text.contains("config 文件"));
        assert!(!text.contains("按 u"));
        assert!(!text.contains("Test1234"));
        assert!(!text.contains("安装 kubectl"));
        assert!(text.contains(&environment::kubeconfig_path().display().to_string()));
        let report = CheckItem::ALL
            .into_iter()
            .map(|item| (item, Ok(CheckDetail::Text(String::new()))))
            .collect::<Vec<_>>();
        environment::record_initialization(&dir, &report).unwrap();
        assert!(matches!(App::new(dir.clone()).screen, Screen::Menu));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reordered_menu_opens_correct_forms() {
        let mut app = app();
        let mut pods = PodHandler::new();
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Fixed(Rect::new(0, 0, 80, 24)),
            },
        )
        .unwrap();
        handle_key(&mut app, &mut terminal, &mut pods, KeyCode::Esc).unwrap();
        assert_eq!(app.selected, MENU_COUNT - 1);
        assert!(matches!(app.screen, Screen::Menu));
        app.selected = 0;
        handle_key(&mut app, &mut terminal, &mut pods, KeyCode::Enter).unwrap();
        assert!(matches!(app.screen, Screen::Form(FormKind::Install)));
        assert_eq!(app.form_values.len(), 3);
        assert_eq!(Lang::Zh.t().menu_labels[4], "更新用户信息");
        assert_eq!(Lang::Zh.t().menu_labels[MENU_COUNT - 1], "检查环境");
        assert_eq!(
            Lang::En.t().menu_labels[MENU_COUNT - 1],
            "Check environment"
        );
    }

    #[test]
    fn report_and_credential_form_show_plaintext_lecture_credentials() {
        let mut app = app();
        let mut info = UserInfo::from_username("student01").unwrap();
        info.context = "lesson".into();
        info.server = "https://fixture.invalid".into();
        info.namespace = "class".into();
        app.screen = Screen::Report(vec![(
            CheckItem::Credentials,
            Ok(CheckDetail::Account(info)),
        )]);
        let text = render(&app);
        assert!(text.contains("用户名: student01"));
        assert!(text.contains("密码: Test1234"));
        assert!(text.contains("https://fixture.invalid"));
        assert!(text.contains("当前 context: lesson"));
        app.toggle_lang();
        assert!(render(&app).contains("Password: Test1234"));

        let mut info = UserInfo::from_username("student01").unwrap();
        info.context = "lesson".into();
        info.server = "https://fixture.invalid".into();
        app.credentials = Some(info);
        app.form_values = vec!["edited".into(), "VisiblePassword".into()];
        app.screen = Screen::Form(FormKind::Credentials);
        assert!(render(&app).contains("VisiblePassword"));
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Fixed(Rect::new(0, 0, 80, 24)),
            },
        )
        .unwrap();
        handle_form_key(
            &mut app,
            &mut terminal,
            &mut PodHandler::new(),
            FormKind::Credentials,
            KeyCode::Enter,
        )
        .unwrap();
        assert!(matches!(app.screen, Screen::Menu));
        assert!(app.credentials.is_none());
        assert!(app.form_values.is_empty());
        assert!(app.config_dir.join("lecture-user.config").is_file());
        std::fs::remove_dir_all(&app.config_dir).unwrap();
    }

    #[test]
    fn progress_displays_current_substep_and_reports_scroll_safely() {
        let app = app();
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 36)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                draw_report(
                    frame,
                    area,
                    &app,
                    &[],
                    Some((CheckItem::Kubectl, SetupStep::DownloadBinary)),
                );
            })
            .unwrap();
        let text = screen_text(&terminal);
        assert!(text.contains("[2/6] kubectl — 下载安装包"));
        assert!(text.contains("等待检查"));
        assert_eq!(scroll_report(0, KeyCode::Up, 20), 0);
        assert_eq!(scroll_report(0, KeyCode::PageDown, 20), 10);
        assert_eq!(scroll_report(19, KeyCode::PageDown, 20), 20);
        assert_eq!(scroll_report(0, KeyCode::End, 20), 20);
        assert_eq!(scroll_report(100, KeyCode::Up, 20), 19);
        assert_eq!(scroll_report(20, KeyCode::Home, 20), 0);
    }

    #[test]
    fn successful_installation_returns_directly_to_menu() {
        let mut app = app();
        let mut pods = PodHandler::new();
        app.begin_form();
        finish_installation(
            &mut app,
            &mut pods,
            "lesson1".into(),
            Ok(vec!["lesson1-abc".into()]),
        );
        assert!(matches!(app.screen, Screen::Menu));
        assert_eq!(app.status, Lang::Zh.t().status_installed);
        assert_eq!(pods.pod_list, vec!["lesson1-abc"]);
        assert!(app.form_values.is_empty());
    }

    #[test]
    fn failed_installation_stays_visible_until_dismissed() {
        let mut app = app();
        let mut pods = PodHandler::new();
        finish_installation(
            &mut app,
            &mut pods,
            "lesson1".into(),
            Err(ThumedError::PodStartupFailed {
                release: "lesson1".into(),
                detail: "lesson1-abc: CrashLoopBackOff".into(),
            }),
        );
        let text = render(&app);
        assert!(text.contains("启动失败"));
        assert!(text.contains("helm uninstall lesson1"));
        assert!(text.contains("CrashLoopBackOff"));
        app.toggle_lang();
        assert!(render(&app).contains("Pod startup failed"));
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Fixed(Rect::new(0, 0, 80, 24)),
            },
        )
        .unwrap();
        // No background recheck or deletion can be triggered by the result screen.
        for key in [KeyCode::Char('r'), KeyCode::Char('y')] {
            handle_key(&mut app, &mut terminal, &mut pods, key).unwrap();
            assert!(matches!(app.screen, Screen::InstallFailed { .. }));
        }
        handle_key(&mut app, &mut terminal, &mut pods, KeyCode::Enter).unwrap();
        assert!(matches!(app.screen, Screen::Menu));
        assert_eq!(app.selected, 3);

        app.lang = Lang::Zh;
        finish_installation(
            &mut app,
            &mut pods,
            "lesson1".into(),
            Err(ThumedError::PodStartupTimeout {
                seconds: 30,
                detail: "lesson1-abc: Pending".into(),
            }),
        );
        let text = render(&app);
        assert!(text.contains("不代表 Pod 已失败"));
        assert!(text.contains("Pending"));
        assert!(matches!(app.screen, Screen::InstallFailed { .. }));
        finish_installation(
            &mut app,
            &mut pods,
            "lesson1".into(),
            Err(ThumedError::CommandFailed {
                cmd: "kubectl get pods".into(),
                stderr: "connection refused".into(),
            }),
        );
        let text = render(&app);
        assert!(text.contains("connection refused"));
        assert!(!text.contains("Pod 启动失败"));
    }

    #[test]
    fn menu_selection_wraps() {
        let mut app = app();

        app.move_selection(-1);
        assert_eq!(app.selected, MENU_COUNT - 1);
        app.move_selection(1);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn menu_labels_cover_every_menu_entry() {
        assert_eq!(Lang::Zh.t().menu_labels.len(), MENU_COUNT);
        assert_eq!(Lang::En.t().menu_details.len(), MENU_COUNT);
    }

    #[test]
    fn move_index_wraps_and_tolerates_empty_lists() {
        assert_eq!(move_index(0, -1, 3), 2);
        assert_eq!(move_index(2, 1, 3), 0);
        assert_eq!(move_index(0, 1, 0), 0);
    }

    #[test]
    fn pod_number_allows_zero_after_first_digit() {
        let mut number = String::new();
        assert!(!append_pod_number(&mut number, '0'));
        assert!(append_pod_number(&mut number, '1'));
        assert!(append_pod_number(&mut number, '0'));
        assert_eq!(number, "10");
    }

    #[test]
    fn toggling_language_retranslates_keyed_status() {
        let mut app = app();
        app.set_status(StatusKey::Installed);
        assert_eq!(app.status, Lang::Zh.t().status_installed);

        app.toggle_lang();

        assert_eq!(app.status, Lang::En.t().status_installed);
    }
}
