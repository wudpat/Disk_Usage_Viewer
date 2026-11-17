# Disk_Usage_Viewer
Elementary Systems Programming, KMITL Project 1/2026

what is Disk_usage_Viewer ?
A small desktop utility written in Rust with an Iced GUI. Pick a folder, see its files ranked by size, select the ones you do not need and delete them after a Yes / Cancel confirmation. Every deletion is logged to a MySQL database and can be reviewed on a History screen.

—Group member— ###honggong group
[Chanakan Kanunghet] [69011320] ||     screens, layout, view() functions, theme switch
[Wudhipat Sripodok] [69011587] ||      file_manager.rs (folder scan, delete), db.rs (MySQL), error handling
[Ahn Kongrit] [69011294] ||.   limit_files, update() logic, unit tests, README

—Theme—
Disk Usage Viewer (theme 3 in the project brief): type or browse to a folder, list its files with sizes, and show the largest files.

—Features—

- Folder selection: type a path, or press Browse… to open a native folder dialog.
Real data: scans the chosen folder and lists every file with its size (B / KB / MB / GB / TB), sorted largest first. Refresh re-scans the folder.
- Show largest N: a number box limits the list to the N largest files (default 10, empty or 0 shows all). The header shows how many files are visible, the total size and how many are selected.
- Delete with confirmation: tick files, press Delete selected, then confirm on a separate screen with Yes, delete or Cancel.
- MySQL delete log: each deleted file (name, path, size, time) is saved to a delete_log table. The History screen shows the log, newest first.
- Error messages on screen: empty path, folder not found, nothing selected, delete failure, or MySQL not reachable all show a red message instead of crashing the app.
- Dark / light theme switch (nice-to-have).

![home](image.png)

![file](image-1.png)

![delete-confirmation](image-2.png)

![history](image-3.png)

## Operating system

Developed and tested on: Windows

## Requirements

- A running MySQL server

## Database setup

The app reads the connection string from the `DATABASE_URL` environment variable. If it is not set, it uses `mysql://root:password@localhost:3306/disk_viewer`.

1. Create the database once:

sql

```sql
CREATE DATABASE disk_viewer;
```

1. (Optional) point the app at your own server:
    - Windows PowerShell: `$env:DATABASE_URL="mysql://user:pass@localhost:3306/disk_viewer"`
    - macOS / Linux: `export DATABASE_URL="mysql://user:pass@localhost:3306/disk_viewer"`

The `delete_log` table is created automatically on first use. If MySQL is not running, the app still works and shows an error on the History screen and after a delete.

## How to run
cargo run

Run the unit tests (no database needed):
cargo test

## What we learned

We learned how Iced's State, Message, `update()` and `view()` structure keeps a GUI easy to reason about, and how to move slow work (folder scans, file deletion, database calls) into `Command::perform` so the window does not freeze. We used `Result`, `match`, `map_err` and `?` instead of `unwrap()` on user input, so wrong paths and database failures become on-screen messages. We also learned to keep logic in small functions such as `limit_files` and `scan_blocking` so they can be tested with `cargo test` without the GUI. A problem we solved was keeping the selection consistent after a re-scan or after the limit changed, by pruning selected files that are no longer present or visible.
