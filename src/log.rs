use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::Mutex,
};

#[derive(Debug)]
pub struct Logger {
    file: Mutex<File>,
}

impl Logger {
    pub fn new(file_path: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .expect("Unable to open log file");

        Logger {
            file: Mutex::new(file),
        }
    }

    pub fn log(&self, message: &str) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", message).expect("Unable to write to log file");
    }
}
