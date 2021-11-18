use crate::event::Event;

pub struct Logger(u64);

impl Logger {
    pub fn new(level: u64) -> Logger {
        Logger(level)
    }

    pub fn log(&self, msg: &str, level: u64) {
        if level <= self.0 {
            Event::Notify(msg).send();
        }
    }
    
    pub fn debug(&self, msg: &str) {
        self.log(msg, 3);
    }

    pub fn verbose(&self, msg: &str) {
        self.log(msg, 2);
    }

    pub fn status(&self, msg: &str) {
        self.log(msg, 1);
    }

    pub fn error(&self, msg: &str) {
        self.log(msg, 0);
    }
}
