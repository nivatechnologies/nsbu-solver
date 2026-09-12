use nsbu_solver::SolverError;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

pub fn publish(output: &Path, name: &str, contents: &str) -> Result<(), SolverError> {
    if contents.len() > crate::config::OUTPUT_CAP {
        return Err(SolverError::ResourceLimit);
    }
    let partial = output.join(format!(".{name}.partial"));
    let final_path = output.join(name);
    if partial.exists() || final_path.exists() {
        return Err(SolverError::InvalidPayload);
    }
    let mut file = File::create(&partial).map_err(|_| SolverError::InvalidPayload)?;
    file.write_all(contents.as_bytes())
        .map_err(|_| SolverError::InvalidPayload)?;
    file.sync_all().map_err(|_| SolverError::InvalidPayload)?;
    fs::rename(&partial, &final_path).map_err(|_| SolverError::InvalidPayload)?;
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
