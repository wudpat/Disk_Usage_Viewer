mod file_manager {
    use std::fs;
    use std::io;

    pub struct SelectedFile {
        path: String,
        specific_file_type: Option<String>,
        files: Vec<fs::DirEntry>
    }
    
    impl SelectedFile {
        pub fn new(input_path: String) -> SelectedFile {
            SelectedFile {
                path: input_path,
                specific_file_type: None,
                files: Vec::new()
            }
        }

        pub fn select_type(&mut self, input_type: String) {
            self.specific_file_type = Some(input_type);
        }

        fn read_folder_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            for file in fs::read_dir(&self.path)? {
                match file {
                    Ok(successful_file) => self.files.push(successful_file),
                    Err(_) => {}
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
    }
}
