mod checksum;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{thread, time};
use std::error::Error;
use std::fs::{ReadDir, Metadata};

use error::PathError;

pub struct DirectoryWatcher {
    /// Indicates how often should the directory be checked.
    secs_interval: u64,
    file_register: HashMap<PathBuf, u32>,
    dir: PathBuf,
    recurisve: bool
}

impl DirectoryWatcher {
    /// Creates a new directory watcher. It must be defined an check interval in milliseconds
    /// and a path to the directory which should be watched.
    pub fn new(secs_interval: u64, path: &str, recurisve: bool) -> Result<DirectoryWatcher, PathError> {
        let path = PathBuf::from(path);
        if !path.is_dir() {
            return Err(PathError);
        }

        Ok(DirectoryWatcher {
            secs_interval,
            file_register: HashMap::new(),
            dir: path,
            recurisve: recurisve
        })
    }
    
    /// Will wait for the setted interval and then emitt the files that were added oder changed.
    pub fn emitted_changed_files(&mut self) -> Result<Vec<String>, Box<Error>> {
        let millis = time::Duration::from_secs(self.secs_interval);
        thread::sleep(millis);
        let dir_clone  = self.dir.clone();
        self.collect_changed_files(&dir_clone)
    }

    /// Collect all changed file in the defined dir and also recurisve when configured.
    fn collect_changed_files(&mut self, path: &Path) -> Result<Vec<String>, Box<Error>> {
        let mut changed_files: Vec<String> = Vec::new();

        let path_cont: ReadDir = path.read_dir()?;
        for dir_entry in path_cont {
            if let Ok(dir_entry) = dir_entry {
                let meta_data: Metadata = dir_entry.metadata()?;
                let path = dir_entry.path();
                if meta_data.is_file() {                
                    let (has_changed, checksum) =  self.has_file_changed(&path);
                    if has_changed {
                        self.register_file(path, checksum);
                        if let Some(path_string) = DirectoryWatcher::convert_to_string(&dir_entry.path()) {
                            changed_files.push(path_string);
                        }
                    }
                } else if meta_data.is_dir() && self.recurisve {
                    let files = self.collect_changed_files(&path)?;
                    changed_files.extend(files);
                }
            }
        }
        Ok(changed_files)
    }

    /// Registers file when it can be converted to a valid &str.
    fn register_file(&mut self, path: PathBuf, checksum: u32) {
        self.file_register.insert(path, checksum);
    }

    /// Checks if the modified time is different
    fn has_file_changed(&self, path: &PathBuf) -> (bool, u32) {
        match self.file_register.get(path) {
            Some(registered_checksum) => checksum::has_file_changed(path, registered_checksum),
            None => (true, checksum::calc_file_checksum(path))
        }
    }

    fn convert_to_string(path: &Path) -> Option<String> {
        match path.as_os_str().to_str() {
            Some(strng) => Some(String::from(strng)),
            None => None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::DirectoryWatcher;
    use std::fs::File;
    use std::io::prelude::*;
    use std::fs::remove_file;

    #[test]
    fn test_new_directorywatcher_path_ok() {
        let test = DirectoryWatcher::new(5, "./test.d", false);
        assert!(test.is_ok());
    }

    #[test]
    fn test_new_directorywatcher_error_path() {
        let test = DirectoryWatcher::new(5, "./test_.d", false);
        assert!(test.is_err());
    }

    #[test]
    fn test_emitted_changed_files_only_one_file_added(){
        let mut test = DirectoryWatcher::new(5, "./test.d/1", false).unwrap();
        for i in 0..5 {
            let files = test.emitted_changed_files().unwrap();
            if i == 0 {
                assert_eq!(1, files.len());
            } else {
                assert_eq!(0, files.len());
            }
        }
    }

    #[test]
    fn test_emitted_changed_files_one_file_added_and_add_another_one_afterwards(){
        let create_file = "test.d/2/test.txt";
        let mut test = DirectoryWatcher::new(5, "./test.d/2", false).unwrap();

        try_delete_file("test.d/2/test.txt");
        for i in 0..5 {
            let files = test.emitted_changed_files().unwrap();
            
            match i {
                0 => {
                        assert_eq!(1, files.len());                    
                        let _ = try_create_file(create_file, b"A simple text");
                    },
                1 => assert_eq!(1, files.len()),
                _ => assert_eq!(0, files.len(), "{:?}", files)
            }
        }
    }

    #[test]
    fn test_emitted_changed_files_one_file_added_and_change_it_afterwards() {
        let create_file = "test.d/4/test.txt";
        let mut test = DirectoryWatcher::new(5, "./test.d/4", false).unwrap();

        let mut file = try_create_file(create_file, b"A simple text");
        for i in 0..5 {
            let files = test.emitted_changed_files().unwrap();            
            match i {
                0 => {
                        assert_eq!(1, files.len());
                        extend_file(&mut file, b"could this be real?");            
                    },
                1 => assert_eq!(1, files.len()),
                _ => assert_eq!(0, files.len(), "{:?}", files)
            }
        }
    }

    #[test]
    fn test_emitted_changed_files_recursivly(){
        let mut test = DirectoryWatcher::new(5, "./test.d/5", true).unwrap();
        for i in 0..5 {
            let files = test.emitted_changed_files().unwrap();
            if i == 0 {
                assert_eq!(4, files.len());
            } else {
                assert_eq!(0, files.len());
            }
        }
    }

    fn try_create_file(path: &str, content: &[u8]) -> File {
        try_delete_file(path);
        let mut file = File::create(path).unwrap();
        let _ = file.write_all(content).unwrap();
        file
    }

    fn try_delete_file(path: &str) {
        let _ = remove_file(path);
    }

    fn extend_file(file: &mut File, content: &[u8]) {
        let _ = file.write_all(content).unwrap();
    }
}
