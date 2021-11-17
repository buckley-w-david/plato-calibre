use std::{error::Error, fmt};

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug, Clone)]
pub struct PlatoCalibreError {
    msg: String,
}

impl PlatoCalibreError {
    pub fn new(message: &str) -> PlatoCalibreError {
        PlatoCalibreError {
            msg: message.to_string(),
        }
    }
}

impl Error for PlatoCalibreError {}

// Generation of an error is completely separate from how it is displayed.
// There's no need to be concerned about cluttering complex logic with the display style.
//
// Note that we don't store any extra info about the errors. This means we can't state
// which string failed to parse without modifying our types to carry that information.
impl fmt::Display for PlatoCalibreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
