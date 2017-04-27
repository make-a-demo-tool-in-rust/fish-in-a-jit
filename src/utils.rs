use std::fs::File;
use std::path::PathBuf;
use std::error::Error;
use std::io::Read;

/// Takes a path to a file and try to read the file into a String
pub fn file_to_string(path: &PathBuf) -> Result<String, Box<Error>> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("[*]: Failed to open {:?}", path);
            return Err(Box::new(e));
        },
    };

    let mut content = String::new();

    if let Err(e) = file.read_to_string(&mut content) {
        error!("[*]: Failed to read {:?}", path);
        return Err(Box::new(e));
    }

    Ok(content)
}
