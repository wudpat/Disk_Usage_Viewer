pub mod file_manager {
    use std::fs;

    #[derive(Debug, Clone)]
    pub struct FileInfo {
        pub name: String,
        pub path: String,
        pub size: u64,
    }

    pub struct SelectedFolder {
        path: String,
        files: Vec<fs::DirEntry>
    }
    
    impl SelectedFolder {
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

        pub fn new(input_path: String) -> SelectedFolder {
            SelectedFolder {
                path: input_path,
                files: Vec::new()
            }
        }

        fn read_folder_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            self.files.clear();
            for file in fs::read_dir(&self.path)? {
                match file {
                    Ok(successful_file) if successful_file.file_type()?.is_file() => self.files.push(successful_file),
                    Ok(_) => {},
                    Err(e) => return Err(e.into())
                }
            }
            Ok(())
        }

        pub fn rank_file_size(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            match self.read_folder_files() {
                Ok(_) => {
                    let mut sort_error = None;
                    self.files.sort_by(|f1, f2| {
                        let f1_size = match f1.metadata() {
                            Ok(m_data) => m_data.len(),
                            Err(e) => {
                                sort_error = Some(e);
                                return std::cmp::Ordering::Equal;
                            }
                        };
                        let f2_size = match f2.metadata() {
                            Ok(m_data) => m_data.len(),
                            Err(e) => {
                                sort_error = Some(e);
                                return std::cmp::Ordering::Equal;
                            }
                        };
                        f2_size.cmp(&f1_size)
                    });
                    if let Some(e) = sort_error {
                        return Err(e.into());
                    }
                    Ok(())
                },
                Err(e) => Err(e)
            }
        }

        pub fn delete_file(&mut self, target_file_paths: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
            for file_path in target_file_paths {
                fs::remove_file(file_path)?
            }
            Ok(())
        }

        pub fn get_files(&self) -> &Vec<fs::DirEntry> {
            &self.files
        }
    }
}
