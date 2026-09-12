use nsbu_solver::SolverError;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

pub fn publish(output: &Path, name: &str, contents: &str) -> Result<(), SolverError> {
    admit_contents(contents)?;
    let partial = output.join(format!(".{name}.partial"));
    let final_path = output.join(name);
    admit_paths(&partial, &final_path)?;
    write_stage(&partial, contents)?;
    publish_stage(output, &partial, &final_path)
}

fn admit_contents(contents: &str) -> Result<(), SolverError> {
    if contents.len() <= crate::config::OUTPUT_CAP {
        Ok(())
    } else {
        Err(SolverError::ResourceLimit)
    }
}

fn admit_paths(partial: &Path, final_path: &Path) -> Result<(), SolverError> {
    if partial.exists() || final_path.exists() {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}

fn write_stage(partial: &Path, contents: &str) -> Result<(), SolverError> {
    let mut file = create_file(partial)?;
    write_contents(&mut file, contents)?;
    sync_file(&file)
}

fn publish_stage(output: &Path, partial: &Path, final_path: &Path) -> Result<(), SolverError> {
    rename(partial, final_path)?;
    sync_directory(output)
}

fn create_file(path: &Path) -> Result<File, SolverError> {
    File::create(path).map_err(|_| SolverError::InvalidPayload)
}

fn write_contents(file: &mut File, contents: &str) -> Result<(), SolverError> {
    file.write_all(contents.as_bytes())
        .map_err(|_| SolverError::InvalidPayload)
}

fn sync_file(file: &File) -> Result<(), SolverError> {
    file.sync_all().map_err(|_| SolverError::InvalidPayload)
}

fn rename(partial: &Path, final_path: &Path) -> Result<(), SolverError> {
    fs::rename(partial, final_path).map_err(|_| SolverError::InvalidPayload)
}

fn sync_directory(output: &Path) -> Result<(), SolverError> {
    File::open(output)
        .and_then(|f| f.sync_all())
        .map_err(|_| SolverError::InvalidPayload)
}

pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn json_escaping_covers_failure_text() {
        assert_eq!(super::escape("a\"b\nc"), "\"a\\\"b\\nc\"");
    }
}
