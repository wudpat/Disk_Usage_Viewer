pub mod file_manager {
    use std::{ffi::OsString, fs::{self, File}, path::PathBuf};

    #[derive(Debug, Clone)]
    pub struct FileInfo {
        pub name: OsString,
        pub path: PathBuf,
        pub size: u64,
    }

    pub struct FileManager {
        path: String,
        files: Vec<FileInfo>,
        history: Vec<FileInfo>
    }
    
    impl FileManager {
        pub fn new(input_path: String) -> FileManager {
            FileManager {
                path: input_path,
                files: Vec::new(),
                history: Vec::new()
            }
        }

        fn read_folder_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.files.clear();
            for file in fs::read_dir(&self.path)? {
                match file {
                    Ok(successful_file) if successful_file.file_type()?.is_file() => self.files.push(FileInfo {
                        name: successful_file.file_name(),
                        path: successful_file.path(),
                        size: successful_file.metadata()?.len()
                    }),
                    Ok(_) => {},
                    Err(e) => return Err(e.into())
                }
            }
            Ok(())
        }

        pub fn rank_file_size(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            match self.read_folder_files() {
                Ok(_) => {
                    self.files.sort_by(|f1, f2| {
                        f2.size.cmp(&f1.size)
                    });
                    Ok(())
                },
                Err(e) => Err(e)
            }
        }

        pub fn delete_file(&mut self, target_files: Vec<FileInfo>) -> Result<(), Box<dyn std::error::Error>> {
            for file in target_files {
                fs::remove_file(&file.path)?;
                self.log(file);
            }
            Ok(())
        }

        fn log(&mut self, target_file: FileInfo) {
            self.history.push(target_file)
        }

        pub fn get_files(&self) -> &Vec<FileInfo> {
            &self.files
        }

        pub fn get_history(&self) -> &Vec<FileInfo> {
            &self.history
        }

        pub fn format_size(bytes: u64) -> String {
            const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
            if bytes < 1024 {
                return format!("{} B", bytes);
            }
            let mut value = bytes as f64;
            let mut unit = 0;
            while value >= 1024.0 && unit < UNITS.len() - 1 {
                value /= 1024.0;
                unit += 1;
            }
            format!("{:.2} {}", value, UNITS[unit])
        }
    }
}
