mod file_manager;

use std::collections::HashSet;

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{executor, Application, Color, Command, Element, Length, Settings, Theme};

use file_manager::file_manager::{FileInfo, SelectedFolder};

fn format_size(bytes: u64) -> String {
    SelectedFolder::format_size(bytes)
}

fn main() -> iced::Result {
    DiskViewer::run(Settings::default())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Files,
    ConfirmDelete,
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    Browse,
    Picked(Option<String>),
    Scan,
    Scanned(Result<Vec<FileInfo>, String>),
    Toggle(String, bool),
    AskDelete,
    ConfirmDelete,
    CancelDelete,
    Deleted(Result<(), String>),
    GoHome,
    ToggleTheme,
    LimitChanged(String),
}

struct DiskViewer {
    screen: Screen,
    path: String,
    files: Vec<FileInfo>,
    selected: HashSet<String>,
    loading: bool,
    error: Option<String>,
    dark: bool,
    limit_input: String,
}

// ---------- background work (uses file_manager unchanged) ----------

/// Read + rank a folder with SelectedFolder and convert to plain data.
fn scan_blocking(path: String) -> Result<Vec<FileInfo>, String> {
    let mut folder = SelectedFolder::new(path);
    folder.rank_file_size().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in folder.get_files() {
        let size = entry
            .metadata()
            .map_err(|e| format!("Cannot read size of {:?}: {}", entry.file_name(), e))?
            .len();
        out.push(FileInfo {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            size,
        });
    }
    Ok(out)
}

async fn scan_folder(path: String) -> Result<Vec<FileInfo>, String> {
    scan_blocking(path)
}

async fn delete_files(path: String, targets: Vec<String>) -> Result<(), String> {
    let mut folder = SelectedFolder::new(path);
    folder.delete_file(&targets).map_err(|e| e.to_string())
}

async fn pick_folder() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose a folder")
        .pick_folder()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
}

// ---------- application ----------

impl Application for DiskViewer {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            DiskViewer {
                screen: Screen::Home,
                path: String::new(),
                files: Vec::new(),
                selected: HashSet::new(),
                loading: false,
                error: None,
                dark: false,
                limit_input: String::from("10"),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Disk Usage Viewer")
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PathChanged(p) => {
                self.path = p;
                Command::none()
            }
            Message::Browse => Command::perform(pick_folder(), Message::Picked),
            Message::Picked(Some(p)) => {
                self.path = p;
                self.update(Message::Scan)
            }
            Message::Picked(None) => Command::none(),
            Message::Scan => {
                let path = self.path.trim().to_string();
                if path.is_empty() {
                    self.error = Some("Please choose or enter a folder path.".to_string());
                    return Command::none();
                }
                self.loading = true;
                self.error = None;
                Command::perform(scan_folder(path), Message::Scanned)
            }
            Message::Scanned(result) => {
                self.loading = false;
                match result {
                    Ok(files) => {
                        let existing: HashSet<&String> = files.iter().map(|f| &f.path).collect();
                        self.selected.retain(|p| existing.contains(p));
                        self.files = files;
                        self.screen = Screen::Files;
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                Command::none()
            }
            Message::Toggle(path, checked) => {
                if checked {
                    self.selected.insert(path);
                } else {
                    self.selected.remove(&path);
                }
                Command::none()
            }
            Message::AskDelete => {
                if self.selected.is_empty() {
                    self.error = Some("Select at least one file first.".to_string());
                } else {
                    self.error = None;
                    self.screen = Screen::ConfirmDelete;
                }
                Command::none()
            }
            Message::CancelDelete => {
                self.screen = Screen::Files;
                Command::none()
            }
            Message::ConfirmDelete => {
                self.loading = true;
                self.error = None;
                let targets: Vec<String> = self.selected.iter().cloned().collect();
                Command::perform(
                    delete_files(self.path.trim().to_string(), targets),
                    Message::Deleted,
                )
            }
            Message::Deleted(result) => {
                // keep any delete error visible, then rescan so the list matches the disk
                self.error = result.err();
                self.selected.clear();
                self.screen = Screen::Files;
                self.loading = true;
                Command::perform(scan_folder(self.path.trim().to_string()), Message::Scanned)
            }
            Message::GoHome => {
                self.screen = Screen::Home;
                self.error = None;
                Command::none()
            }
            Message::LimitChanged(s) => {
                // keep digits only; empty or 0 means "show all"
                self.limit_input = s.chars().filter(|c| c.is_ascii_digit()).collect();
                // never keep hidden files selected, so they can't be deleted by accident
                let visible: HashSet<String> =
                    self.visible_files().iter().map(|f| f.path.clone()).collect();
                self.selected.retain(|p| visible.contains(p));
                Command::none()
            }
            Message::ToggleTheme => {
                self.dark = !self.dark;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let body = match self.screen {
            Screen::Home => self.home_view(),
            Screen::Files => self.files_view(),
            Screen::ConfirmDelete => self.confirm_view(),
        };
        container(body)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn error_text(error: &Option<String>) -> Element<'_, Message> {
    match error {
        Some(e) => text(e.as_str())
            .style(Color::from_rgb(0.85, 0.1, 0.1))
            .into(),
        None => text("").into(),
    }
}

fn loading_text(loading: bool) -> Element<'static, Message> {
    text(if loading { "Loading..." } else { "" }).into()
}

impl DiskViewer {
    /// The largest N files (files are already sorted largest first).
    /// Empty, 0 or invalid input shows all files.
    fn visible_files(&self) -> &[FileInfo] {
        match self.limit_input.parse::<usize>() {
            Ok(n) if n > 0 => &self.files[..n.min(self.files.len())],
            _ => &self.files,
        }
    }

    fn theme_button(&self) -> Element<'_, Message> {
        let label = if self.dark { "Light theme" } else { "Dark theme" };
        button(label).on_press(Message::ToggleTheme).into()
    }

    fn home_view(&self) -> Element<'_, Message> {
        let mut browse_btn = button("Browse...");
        let mut scan_btn = button("Scan");
        if !self.loading {
            browse_btn = browse_btn.on_press(Message::Browse);
            scan_btn = scan_btn.on_press(Message::Scan);
        }

        column![
            text("Disk Usage Viewer").size(32),
            text("Choose a folder to list its files by size."),
            row![
                text_input("e.g. C:\\Users\\me\\Downloads or /home/me", &self.path)
                    .on_input(Message::PathChanged)
                    .on_submit(Message::Scan)
                    .padding(10),
                browse_btn,
            ]
            .spacing(10),
            row![scan_btn, self.theme_button()].spacing(10),
            loading_text(self.loading),
            error_text(&self.error),
        ]
        .spacing(15)
        .into()
    }

    fn files_view(&self) -> Element<'_, Message> {
        let total: u64 = self.files.iter().map(|f| f.size).sum();

        let mut list = column![].spacing(6);
        for f in self.visible_files() {
            let label = format!("{}   ({})", f.name, format_size(f.size));
            let path = f.path.clone();
            list = list.push(
                checkbox(label, self.selected.contains(&f.path))
                    .on_toggle(move |c| Message::Toggle(path.clone(), c)),
            );
        }
        if self.files.is_empty() && !self.loading {
            list = list.push(text("No files in this folder."));
        }

        let mut delete_btn = button("Delete selected");
        let mut refresh_btn = button("Refresh");
        if !self.loading {
            refresh_btn = refresh_btn.on_press(Message::Scan);
            if !self.selected.is_empty() {
                delete_btn = delete_btn.on_press(Message::AskDelete);
            }
        }

        column![
            text("Files (largest first)").size(28),
            text(format!(
                "{}  |  showing {} of {} files, {} total, {} selected",
                self.path,
                self.visible_files().len(),
                self.files.len(),
                format_size(total),
                self.selected.len()
            )),
            row![
                text("Show largest:"),
                text_input("all", &self.limit_input)
                    .on_input(Message::LimitChanged)
                    .width(Length::Fixed(80.0))
                    .padding(6),
                text("files (empty = all)"),
            ]
            .spacing(10),
            row![
                button("Back").on_press(Message::GoHome),
                refresh_btn,
                delete_btn
            ]
            .spacing(10),
            loading_text(self.loading),
            error_text(&self.error),
            scrollable(list).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(12)
        .into()
    }

    fn confirm_view(&self) -> Element<'_, Message> {
        let mut names = column![].spacing(4);
        for f in self.files.iter().filter(|f| self.selected.contains(&f.path)) {
            names = names.push(text(format!("{} ({})", f.name, format_size(f.size))));
        }
        let mut yes_btn = button("Yes, delete");
        let mut cancel_btn = button("Cancel");
        if !self.loading {
            yes_btn = yes_btn.on_press(Message::ConfirmDelete);
            cancel_btn = cancel_btn.on_press(Message::CancelDelete);
        }

        column![
            text("Confirm delete").size(28),
            text(format!(
                "Delete {} file(s)? This cannot be undone.",
                self.selected.len()
            )),
            scrollable(names).height(Length::Fill),
            loading_text(self.loading),
            row![yes_btn, cancel_btn].spacing(10),
        ]
        .spacing(15)
        .into()
    }
}