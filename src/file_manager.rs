mod file_manager {
    use std::fs;

    pub struct SelectedFolder {
        path: String,
        pub files: Vec<fs::DirEntry>
    }
    
    impl SelectedFolder {
        pub fn new(input_path: String) -> SelectedFolder {
            SelectedFolder {
                path: input_path,
                files: Vec::new()
            }
        }

        fn read_folder_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            for file in fs::read_dir(&self.path)? {
                match file {
                    Ok(successful_file) => self.files.push(successful_file),
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

        pub fn delete_file(&mut self, target_file_paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
            for file_path in target_file_paths {
                fs::remove_file(file_path)?
            }
            Ok(())
        }
    }
}
