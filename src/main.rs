mod db;
mod file_manager;

use std::collections::HashSet;
use std::path::PathBuf;

use iced::widget::{button, checkbox, column, container, row, scrollable, text, text_input};
use iced::{executor, Application, Color, Command, Element, Length, Settings, Theme};

use db::LogEntry;
use file_manager::file_manager::{FileInfo, FileManager};

fn main() -> iced::Result {
    DiskViewer::run(Settings::default())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Screen {
    Home,
    Files,
    ConfirmDelete,
    History,
}

#[derive(Debug, Clone)]
enum Message {
    PathChanged(String),
    Browse,
    Picked(Option<String>),
    Scan,
    Scanned(Result<Vec<FileInfo>, String>),
    Toggle(PathBuf, bool),
    AskDelete,
    ConfirmDelete,
    CancelDelete,
    Deleted(Result<(), String>),
    GoHome,
    ToggleTheme,
    LimitChanged(String),
    ShowHistory,
    HistoryLoaded(Result<Vec<LogEntry>, String>),
}

struct DiskViewer {
    screen: Screen,
    path: String,
    files: Vec<FileInfo>,
    selected: HashSet<PathBuf>,
    loading: bool,
    error: Option<String>,
    dark: bool,
    limit_input: String,
    history: Vec<LogEntry>,
}

fn name_of(f: &FileInfo) -> String {
    f.name.to_string_lossy().into_owned()
}

fn limit_files<'a>(files: &'a [FileInfo], input: &str) -> &'a [FileInfo] {
    match input.parse::<usize>() {
        Ok(n) if n > 0 => &files[..n.min(files.len())],
        _ => files,
    }
}

fn scan_blocking(path: String) -> Result<Vec<FileInfo>, String> {
    let mut folder = FileManager::new(path);
    folder.rank_file_size().map_err(|e| e.to_string())?;
    Ok(folder.get_files().clone())
}

fn delete_blocking(path: String, targets: Vec<FileInfo>) -> Result<(), String> {
    let mut folder = FileManager::new(path);
    let mut log_error = None;
    for file in targets {
        folder
            .delete_file(vec![file.clone()])
            .map_err(|e| format!("Cannot delete {}: {}", name_of(&file), e))?;
        let path = file.path.to_string_lossy();
        if let Err(e) = db::log_delete(&name_of(&file), &path, file.size) {
            log_error = Some(format!("Deleted, but log not saved. {}", e));
        }
    }
    log_error.map_or(Ok(()), Err)
}

async fn pick_folder() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose a folder")
        .pick_folder()
        .await
        .map(|h| h.path().to_string_lossy().into_owned())
}

impl DiskViewer {
    fn visible_files(&self) -> &[FileInfo] {
        limit_files(&self.files, &self.limit_input)
    }

    fn selected_files(&self) -> Vec<FileInfo> {
        self.files
            .iter()
            .filter(|f| self.selected.contains(&f.path))
            .cloned()
            .collect()
    }

    fn scan_command(&mut self) -> Command<Message> {
        self.loading = true;
        let path = self.path.trim().to_string();
        Command::perform(async move { scan_blocking(path) }, Message::Scanned)
    }
}

impl Application for DiskViewer {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let app = DiskViewer {
            screen: Screen::Home,
            path: String::new(),
            files: Vec::new(),
            selected: HashSet::new(),
            loading: false,
            error: None,
            dark: false,
            limit_input: String::from("10"),
            history: Vec::new(),
        };
        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("Disk Usage Viewer")
    }

    fn theme(&self) -> Theme {
        if self.dark { Theme::Dark } else { Theme::Light }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PathChanged(p) => self.path = p,
            Message::Browse => return Command::perform(pick_folder(), Message::Picked),
            Message::Picked(Some(p)) => {
                self.path = p;
                return self.update(Message::Scan);
            }
            Message::Picked(None) => {}
            Message::Scan => {
                if self.path.trim().is_empty() {
                    self.error = Some("Please choose or enter a folder path.".to_string());
                } else {
                    self.error = None;
                    return self.scan_command();
                }
            }
            Message::Scanned(result) => {
                self.loading = false;
                match result {
                    Ok(files) => {
                        let existing: HashSet<&PathBuf> = files.iter().map(|f| &f.path).collect();
                        self.selected.retain(|p| existing.contains(p));
                        self.files = files;
                        self.screen = Screen::Files;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            Message::Toggle(path, checked) => {
                if checked {
                    self.selected.insert(path);
                } else {
                    self.selected.remove(&path);
                }
            }
            Message::AskDelete => {
                if self.selected.is_empty() {
                    self.error = Some("Select at least one file first.".to_string());
                } else {
                    self.error = None;
                    self.screen = Screen::ConfirmDelete;
                }
            }
            Message::CancelDelete => self.screen = Screen::Files,
            Message::ConfirmDelete => {
                self.loading = true;
                self.error = None;
                let path = self.path.trim().to_string();
                let targets = self.selected_files();
                return Command::perform(
                    async move { delete_blocking(path, targets) },
                    Message::Deleted,
                );
            }
            Message::Deleted(result) => {
                self.error = result.err();
                self.selected.clear();
                self.screen = Screen::Files;
                return self.scan_command();
            }
            Message::GoHome => {
                self.screen = Screen::Home;
                self.error = None;
            }
            Message::LimitChanged(s) => {
                self.limit_input = s.chars().filter(|c| c.is_ascii_digit()).collect();
                let visible: HashSet<PathBuf> =
                    self.visible_files().iter().map(|f| f.path.clone()).collect();
                self.selected.retain(|p| visible.contains(p));
            }
            Message::ToggleTheme => self.dark = !self.dark,
            Message::ShowHistory => {
                self.screen = Screen::History;
                self.loading = true;
                self.error = None;
                return Command::perform(
                    async { db::load_history() },
                    Message::HistoryLoaded,
                );
            }
            Message::HistoryLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(entries) => self.history = entries,
                    Err(e) => self.error = Some(e),
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let body = match self.screen {
            Screen::Home => self.home_view(),
            Screen::Files => self.files_view(),
            Screen::ConfirmDelete => self.confirm_view(),
            Screen::History => self.history_view(),
        };
        container(body)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn error_text(error: &Option<String>) -> Element<'_, Message> {
    text(error.as_deref().unwrap_or(""))
        .style(Color::from_rgb(0.85, 0.1, 0.1))
        .into()
}

fn loading_text(loading: bool) -> Element<'static, Message> {
    text(if loading { "Loading..." } else { "" }).into()
}

fn action_button(label: &str, msg: Message, enabled: bool) -> Element<'_, Message> {
    let b = button(label);
    if enabled { b.on_press(msg).into() } else { b.into() }
}

impl DiskViewer {
    fn home_view(&self) -> Element<'_, Message> {
        let idle = !self.loading;
        let theme_label = if self.dark { "Light theme" } else { "Dark theme" };

        column![
            text("Disk Usage Viewer").size(32),
            text("Choose a folder to list its files by size."),
            row![
                text_input("e.g. C:\\Users\\me\\Downloads or /home/me", &self.path)
                    .on_input(Message::PathChanged)
                    .on_submit(Message::Scan)
                    .padding(10),
                action_button("Browse...", Message::Browse, idle),
            ]
            .spacing(10),
            row![
                action_button("Scan", Message::Scan, idle),
                action_button("History", Message::ShowHistory, idle),
                action_button(theme_label, Message::ToggleTheme, true),
            ]
            .spacing(10),
            loading_text(self.loading),
            error_text(&self.error),
        ]
        .spacing(15)
        .into()
    }

    fn files_view(&self) -> Element<'_, Message> {
        let total: u64 = self.files.iter().map(|f| f.size).sum();
        let idle = !self.loading;

        let mut list = column![].spacing(6);
        for f in self.visible_files() {
            let label = format!("{}   ({})", name_of(f), FileManager::format_size(f.size));
            let path = f.path.clone();
            list = list.push(
                checkbox(label, self.selected.contains(&f.path))
                    .on_toggle(move |c| Message::Toggle(path.clone(), c)),
            );
        }
        if self.files.is_empty() && idle {
            list = list.push(text("No files in this folder."));
        }

        column![
            text("Files (largest first)").size(28),
            text(format!(
                "{}  |  showing {} of {} files, {} total, {} selected",
                self.path,
                self.visible_files().len(),
                self.files.len(),
                FileManager::format_size(total),
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
                action_button("Back", Message::GoHome, true),
                action_button("Refresh", Message::Scan, idle),
                action_button(
                    "Delete selected",
                    Message::AskDelete,
                    idle && !self.selected.is_empty()
                ),
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
        let idle = !self.loading;
        let mut names = column![].spacing(4);
        for f in self.selected_files() {
            names = names.push(text(format!(
                "{} ({})",
                name_of(&f),
                FileManager::format_size(f.size)
            )));
        }

        column![
            text("Confirm delete").size(28),
            text(format!(
                "Delete {} file(s)? This cannot be undone.",
                self.selected.len()
            )),
            scrollable(names).height(Length::Fill),
            loading_text(self.loading),
            row![
                action_button("Yes, delete", Message::ConfirmDelete, idle),
                action_button("Cancel", Message::CancelDelete, idle),
            ]
            .spacing(10),
        ]
        .spacing(15)
        .into()
    }

    fn history_view(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(6);
        for h in &self.history {
            list = list.push(text(format!(
                "{}  |  {} ({})  |  {}",
                h.deleted_at,
                h.file_name,
                FileManager::format_size(h.size_bytes),
                h.file_path
            )));
        }
        if self.history.is_empty() && !self.loading && self.error.is_none() {
            list = list.push(text("No deleted files logged yet."));
        }

        column![
            text("Delete history ").size(28),
            row![
                action_button("Back", Message::GoHome, true),
                action_button("Refresh", Message::ShowHistory, !self.loading),
            ]
            .spacing(10),
            loading_text(self.loading),
            error_text(&self.error),
            scrollable(list).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(12)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fi(name: &str, size: u64) -> FileInfo {
        FileInfo {
            name: name.into(),
            path: PathBuf::from(name),
            size,
        }
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(FileManager::format_size(0), "0 B");
        assert_eq!(FileManager::format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_units() {
        assert_eq!(FileManager::format_size(1024), "1.00 KB");
        assert_eq!(FileManager::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(FileManager::format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn format_size_caps_at_tb() {
        assert_eq!(FileManager::format_size(1024u64.pow(5)), "1024.00 TB");
    }

    #[test]
    fn limit_files_applies_limit() {
        let files = vec![fi("a", 3), fi("b", 2), fi("c", 1)];
        assert_eq!(limit_files(&files, "2").len(), 2);
        assert_eq!(limit_files(&files, "99").len(), 3);
    }

    #[test]
    fn limit_files_empty_or_zero_means_all() {
        let files = vec![fi("a", 3), fi("b", 2)];
        assert_eq!(limit_files(&files, "").len(), 2);
        assert_eq!(limit_files(&files, "0").len(), 2);
    }

    #[test]
    fn scan_sorts_largest_first() {
        let dir = std::env::temp_dir().join("disk_viewer_test_scan");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("small.txt"), vec![0u8; 10]).unwrap();
        fs::write(dir.join("big.txt"), vec![0u8; 1000]).unwrap();
        fs::write(dir.join("mid.txt"), vec![0u8; 100]).unwrap();

        let files = scan_blocking(dir.to_string_lossy().into_owned()).unwrap();
        let sizes: Vec<u64> = files.iter().map(|f| f.size).collect();
        assert_eq!(sizes, vec![1000, 100, 10]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_missing_folder_returns_error() {
        assert!(scan_blocking("/this/folder/does/not/exist_xyz".to_string()).is_err());
    }

    #[test]
    fn delete_file_removes_it_from_disk() {
        let dir = std::env::temp_dir().join("disk_viewer_test_delete");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.txt");
        fs::write(&p, b"hello").unwrap();

        let mut fm = FileManager::new(dir.to_string_lossy().into_owned());
        let target = FileInfo {
            name: "x.txt".into(),
            path: p.clone(),
            size: 5,
        };
        assert!(fm.delete_file(vec![target]).is_ok());
        assert!(!p.exists());
        assert_eq!(fm.get_history().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}