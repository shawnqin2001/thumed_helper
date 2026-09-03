use crate::constants;
use crate::environment::{self, DirManager, UserInfo};
use crate::error::{Result, ThumedError};
use crate::pod_handler::{PodConfig, PodHandler};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

const MENU_ITEMS: [MenuItem; 7] = [
    MenuItem {
        label: "Check environment",
        detail: "Verify saved credentials, kubectl, Helm, and Helm repository.",
    },
    MenuItem {
        label: "List pods",
        detail: "Refresh Kubernetes pod list.",
    },
    MenuItem {
        label: "Install pod",
        detail: "Create a pod configuration and install it with Helm.",
    },
    MenuItem {
        label: "Login to pod",
        detail: "Choose a pod, then open its shell.",
    },
    MenuItem {
        label: "Forward pod",
        detail: "Choose a pod, then forward port 8787.",
    },
    MenuItem {
        label: "Uninstall pod",
        detail: "Choose a pod, then confirm Helm release removal.",
    },
    MenuItem {
        label: "Update user information",
        detail: "Replace saved cluster credentials.",
    },
];

#[derive(Clone, Copy)]
struct MenuItem {
    label: &'static str,
    detail: &'static str,
}

#[derive(Clone, Copy)]
enum PodAction {
    Login,
    Forward,
    Uninstall,
}

impl PodAction {
    fn title(self) -> &'static str {
        match self {
            Self::Login => "Login to pod",
            Self::Forward => "Forward pod",
            Self::Uninstall => "Uninstall pod",
        }
    }
}

#[derive(Clone, Copy)]
enum FormKind {
    Install,
    Credentials,
}

impl FormKind {
    fn title(self) -> &'static str {
        match self {
            Self::Install => "Install pod",
            Self::Credentials => "Update user information",
        }
    }

    fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Install => &[
                "Pod name",
                "CPU cores (blank = default)",
                "Memory GB (blank = default)",
            ],
            Self::Credentials => &["Username", "Password"],
        }
    }
}

enum Screen {
    Menu,
    PodPicker(PodAction),
    Form(FormKind),
    ConfirmUninstall { pod_name: String, release: String },
}

struct App {
    selected: usize,
    pod_selected: usize,
    pod_number: String,
    form_selected: usize,
    form_values: Vec<String>,
    screen: Screen,
    status: String,
}

impl App {
    fn new(dirman: &DirManager) -> Self {
        let status = if dirman.bin_dir.exists() {
            "Loading pods...".to_string()
        } else {
            "Environment is not initialized. Select Check environment.".to_string()
        };
        Self {
            selected: 0,
            pod_selected: 0,
            pod_number: String::new(),
            form_selected: 0,
            form_values: Vec::new(),
            screen: Screen::Menu,
            status,
        }
    }

    fn move_selection(&mut self, delta: isize) {
        self.selected = move_index(self.selected, delta, MENU_ITEMS.len());
    }

    fn begin_form(&mut self, kind: FormKind) {
        self.form_selected = 0;
        self.form_values = vec![String::new(); kind.labels().len()];
        self.screen = Screen::Form(kind);
    }

    fn begin_pod_picker(&mut self, action: PodAction) {
        self.pod_selected = 0;
        self.pod_number.clear();
        self.screen = Screen::PodPicker(action);
    }
}

fn move_index(index: usize, delta: isize, count: usize) -> usize {
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
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

pub fn run(dirman: &DirManager, pod_handler: &mut PodHandler) -> Result<()> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let mut app = App::new(dirman);

    terminal.draw(|frame| draw_ui(frame, &app, pod_handler))?;
    if let Err(error) = environment::add_path(&dirman.bin_dir) {
        app.status = format!("PATH error: {}", error);
    } else {
        refresh_pods(&mut app, pod_handler);
    }

    loop {
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
        if matches!(&app.screen, Screen::Menu)
            && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        {
            break Ok(());
        }

        if let Err(error) = handle_key(&mut app, &mut terminal, dirman, pod_handler, key.code) {
            app.status = format!("Error: {}", error);
        }
    }
}

fn handle_key(
    app: &mut App,
    terminal: &mut AppTerminal,
    dirman: &DirManager,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    match app.screen {
        Screen::Menu => handle_menu_key(app, terminal, dirman, pod_handler, key),
        Screen::PodPicker(action) => handle_pod_picker_key(app, terminal, pod_handler, action, key),
        Screen::Form(kind) => handle_form_key(app, dirman, pod_handler, kind, key),
        Screen::ConfirmUninstall { .. } => handle_uninstall_confirmation(app, pod_handler, key),
    }
}

fn handle_menu_key(
    app: &mut App,
    _terminal: &mut AppTerminal,
    dirman: &DirManager,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {}
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Enter => match app.selected {
            0 => {
                environment::check_env(dirman)?;
                app.status = "Environment check completed.".to_string();
            }
            1 => refresh_pods(app, pod_handler),
            2 => app.begin_form(FormKind::Install),
            3 => open_pod_picker(app, pod_handler, PodAction::Login),
            4 => open_pod_picker(app, pod_handler, PodAction::Forward),
            5 => open_pod_picker(app, pod_handler, PodAction::Uninstall),
            6 => app.begin_form(FormKind::Credentials),
            _ => unreachable!(),
        },
        _ => {}
    }
    Ok(())
}

fn open_pod_picker(app: &mut App, pod_handler: &mut PodHandler, action: PodAction) {
    refresh_pods(app, pod_handler);
    if pod_handler.pod_list.is_empty() {
        app.status = "No pods available. Create or refresh a pod first.".to_string();
    } else {
        app.begin_pod_picker(action);
    }
}

fn refresh_pods(app: &mut App, pod_handler: &mut PodHandler) {
    match pod_handler.get_pod_list() {
        Ok(()) => app.status = format!("Loaded {} pod(s).", pod_handler.pod_list.len()),
        Err(error) => app.status = format!("Could not load pods: {}", error),
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
                if number <= count {
                    app.pod_selected = number - 1;
                }
            }
        }
        KeyCode::Enter => {
            if !app.pod_number.is_empty() {
                let number = app.pod_number.parse::<usize>().unwrap_or(0);
                if number == 0 || number > count {
                    app.status = format!("Pod number must be between 1 and {}.", count);
                    return Ok(());
                }
                app.pod_selected = number - 1;
            }
            let pod_name = pod_handler.pod_list[app.pod_selected].clone();
            match action {
                PodAction::Login => {
                    app.screen = Screen::Menu;
                    with_terminal_suspended(terminal, || pod_handler.login_pod_by_name(&pod_name))?;
                    app.status = "Pod login session ended.".to_string();
                }
                PodAction::Forward => {
                    app.screen = Screen::Menu;
                    with_terminal_suspended(terminal, || {
                        pod_handler.forward_pod_by_name(&pod_name)
                    })?;
                    app.status = "Port-forward session ended.".to_string();
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
    dirman: &DirManager,
    pod_handler: &mut PodHandler,
    kind: FormKind,
    key: KeyCode,
) -> Result<()> {
    match key {
        KeyCode::Esc => app.screen = Screen::Menu,
        KeyCode::Up => app.form_selected = move_index(app.form_selected, -1, app.form_values.len()),
        KeyCode::Down | KeyCode::Tab => {
            app.form_selected = move_index(app.form_selected, 1, app.form_values.len())
        }
        KeyCode::Backspace => {
            app.form_values[app.form_selected].pop();
        }
        KeyCode::Char(character) => app.form_values[app.form_selected].push(character),
        KeyCode::Enter => submit_form(app, dirman, pod_handler, kind)?,
        _ => {}
    }
    Ok(())
}

fn submit_form(
    app: &mut App,
    dirman: &DirManager,
    pod_handler: &mut PodHandler,
    kind: FormKind,
) -> Result<()> {
    match kind {
        FormKind::Install => {
            let pod_config = PodConfig::from_values(
                &app.form_values[0],
                &app.form_values[1],
                &app.form_values[2],
            )?;
            pod_config.install_pod(dirman)?;
            refresh_pods(app, pod_handler);
            app.status = "Pod installed successfully.".to_string();
        }
        FormKind::Credentials => {
            let user = app.form_values[0].trim();
            let password = app.form_values[1].trim();
            if user.is_empty() || password.is_empty() {
                return Err(ThumedError::Config(
                    "Username and password are required.".to_string(),
                ));
            }
            UserInfo::new(user.to_string(), password.to_string()).save(dirman)?;
            app.status = "User information updated.".to_string();
        }
    }
    app.screen = Screen::Menu;
    Ok(())
}

fn handle_uninstall_confirmation(
    app: &mut App,
    pod_handler: &mut PodHandler,
    key: KeyCode,
) -> Result<()> {
    let Screen::ConfirmUninstall { pod_name, release } = &app.screen else {
        unreachable!();
    };
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let pod_name = pod_name.clone();
            let release = release.clone();
            pod_handler.uninstall_pod_release(&pod_name, &release)?;
            app.status = format!("Uninstalled release {} for {}.", release, pod_name);
            app.screen = Screen::Menu;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.status = "Uninstall cancelled.".to_string();
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
        Span::raw(format!(" {}", constants::APP_VERSION)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, layout[0]);

    match &app.screen {
        Screen::Menu => draw_menu(frame, layout[1], app, pod_handler),
        Screen::PodPicker(action) => draw_pod_picker(frame, layout[1], app, pod_handler, *action),
        Screen::Form(kind) => draw_form(frame, layout[1], app, *kind),
        Screen::ConfirmUninstall { pod_name, release } => {
            draw_uninstall_confirmation(frame, layout[1], app, pod_name, release)
        }
    }

    let footer = match app.screen {
        Screen::Menu => "↑/↓ or j/k navigate   Enter select   q/Esc quit",
        Screen::PodPicker(_) => "↑/↓ or j/k select   1-99 choose number   Enter confirm   Esc back",
        Screen::Form(_) => "Type values   ↑/↓/Tab switch field   Enter submit   Esc cancel",
        Screen::ConfirmUninstall { .. } => "y confirm uninstall   n/Esc cancel",
    };
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)),
        layout[2],
    );
}

fn draw_menu(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, pod_handler: &PodHandler) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);
    let items = MENU_ITEMS
        .iter()
        .map(|item| ListItem::new(item.label))
        .collect::<Vec<_>>();
    let menu = List::new(items)
        .block(Block::default().title("Actions").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    let mut menu_state = ListState::default();
    menu_state.select(Some(app.selected));
    frame.render_stateful_widget(menu, columns[0], &mut menu_state);

    let selected = MENU_ITEMS[app.selected];
    let mut details = vec![
        Line::from(Span::styled(
            selected.detail,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Pods", Style::default().fg(Color::Yellow))),
    ];
    if pod_handler.pod_list.is_empty() {
        details.push(Line::from("No pods loaded."));
    } else {
        for (index, pod) in pod_handler.pod_list.iter().take(12).enumerate() {
            details.push(Line::from(format!("{:>2}. {}", index + 1, pod)));
        }
        if pod_handler.pod_list.len() > 12 {
            details.push(Line::from(format!(
                "... and {} more",
                pod_handler.pod_list.len() - 12
            )));
        }
    }
    details.push(Line::from(""));
    details.push(status_line(app));
    frame.render_widget(
        Paragraph::new(details)
            .block(Block::default().title("Details").borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        columns[1],
    );
}

fn draw_pod_picker(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
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
        action.title().to_string()
    } else {
        format!("{} — number: {}", action.title(), app.pod_number)
    };
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    let mut state = ListState::default();
    state.select(Some(app.pod_selected));
    frame.render_stateful_widget(list, sections[0], &mut state);
    frame.render_widget(Paragraph::new(status_line(app)), sections[1]);
}

fn draw_form(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, kind: FormKind) {
    let mut lines = vec![
        Line::from("Enter values, then press Enter to submit."),
        Line::from(""),
    ];
    for (index, label) in kind.labels().iter().enumerate() {
        let value = if matches!(kind, FormKind::Credentials) && index == 1 {
            "•".repeat(app.form_values[index].chars().count())
        } else {
            app.form_values[index].clone()
        };
        let marker = if index == app.form_selected {
            "▶"
        } else {
            " "
        };
        lines.push(Line::from(Span::styled(
            format!("{} {}: {}", marker, label, value),
            if index == app.form_selected {
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
            .block(Block::default().title(kind.title()).borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_uninstall_confirmation(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &App,
    pod_name: &str,
    release: &str,
) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "This removes Helm release.",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("Pod: {}", pod_name)),
            Line::from(format!("Helm release: {}", release)),
            Line::from(""),
            Line::from("Continue? (y/n)"),
            status_line(app),
        ])
        .block(
            Block::default()
                .title("Confirm uninstall")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn status_line(app: &App) -> Line<'static> {
    Line::from(vec![
        Span::styled("Status: ", Style::default().fg(Color::Yellow)),
        Span::raw(app.status.clone()),
    ])
}

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
    use std::path::PathBuf;

    #[test]
    fn menu_selection_wraps() {
        let dirman = DirManager {
            config_dir: PathBuf::from("/tmp/config"),
            bin_dir: PathBuf::from("/tmp/bin"),
        };
        let mut app = App::new(&dirman);

        app.move_selection(-1);
        assert_eq!(app.selected, MENU_ITEMS.len() - 1);
        app.move_selection(1);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn move_index_wraps_pod_selection() {
        assert_eq!(move_index(0, -1, 3), 2);
        assert_eq!(move_index(2, 1, 3), 0);
    }

    #[test]
    fn pod_number_allows_zero_after_first_digit() {
        let mut number = String::new();
        assert!(!append_pod_number(&mut number, '0'));
        assert!(append_pod_number(&mut number, '1'));
        assert!(append_pod_number(&mut number, '0'));
        assert_eq!(number, "10");
    }
}
