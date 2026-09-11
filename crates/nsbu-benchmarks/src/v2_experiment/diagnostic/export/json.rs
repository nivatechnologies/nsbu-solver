use super::DiagnosticExportError;
use std::{
    fmt,
    io::{self, Write},
};

pub(super) struct Json<W> {
    writer: W,
    bytes: usize,
    calls: usize,
    error: Option<io::Error>,
}
impl<W: Write> Json<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            bytes: 0,
            calls: 0,
            error: None,
        }
    }
    pub fn raw(&mut self, value: &str) -> Result<(), DiagnosticExportError> {
        self.write_all(value.as_bytes())
            .map_err(|_| self.take_error())
    }
    pub fn string(&mut self, value: &str) -> Result<(), DiagnosticExportError> {
        if value.bytes().any(|b| b < 0x20 || matches!(b, b'"' | b'\\')) {
            return Err(DiagnosticExportError::InvalidReport);
        }
        self.raw("\"")?;
        self.raw(value)?;
        self.raw("\"")
    }
    pub fn usize(&mut self, value: usize) -> Result<(), DiagnosticExportError> {
        self.args(format_args!("{value}"))
    }
    pub fn counter(&mut self, value: usize) -> Result<(), DiagnosticExportError> {
        self.args(format_args!("\"{value}\""))
    }
    pub fn u128(&mut self, value: u128) -> Result<(), DiagnosticExportError> {
        self.args(format_args!("\"{value}\""))
    }
    pub fn i32(&mut self, value: i32) -> Result<(), DiagnosticExportError> {
        self.args(format_args!("{value}"))
    }
    pub fn bool(&mut self, value: bool) -> Result<(), DiagnosticExportError> {
        self.raw(if value { "true" } else { "false" })
    }
    pub fn f64(&mut self, value: f64) -> Result<(), DiagnosticExportError> {
        if !value.is_finite() {
            return Err(DiagnosticExportError::InvalidReport);
        }
        self.args(format_args!("{value:.17e}"))
    }
    pub fn hex(&mut self, value: [u8; 32]) -> Result<(), DiagnosticExportError> {
        self.raw("\"")?;
        for byte in value {
            self.args(format_args!("{byte:02x}"))?;
        }
        self.raw("\"")
    }
    pub fn args(&mut self, args: fmt::Arguments<'_>) -> Result<(), DiagnosticExportError> {
        match self.write_fmt(args) {
            Ok(()) => Ok(()),
            Err(_) => Err(self.take_error()),
        }
    }
    pub fn finish(mut self) -> Result<(usize, usize, W), DiagnosticExportError> {
        if self.error.is_some() {
            return Err(self.take_error());
        }
        Ok((self.bytes, self.calls, self.writer))
    }
    fn take_error(&mut self) -> DiagnosticExportError {
        DiagnosticExportError::Io {
            validation_bytes: 0,
            validation_calls: 0,
            bytes_written: self.bytes,
            write_calls: self.calls,
            source: self
                .error
                .take()
                .unwrap_or_else(|| io::Error::other("JSON writer failed")),
        }
    }
}
impl<W: Write> Write for Json<W> {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let initial = buf.len();
        while !buf.is_empty() {
            self.calls = self
                .calls
                .checked_add(1)
                .ok_or_else(|| io::Error::other("write call counter overflow"))?;
            match self.writer.write(buf) {
                Ok(0) => {
                    let e = io::Error::new(io::ErrorKind::WriteZero, "failed to write JSON");
                    self.error = Some(io::Error::new(e.kind(), e.to_string()));
                    return Err(e);
                }
                Ok(n) => {
                    self.bytes = self
                        .bytes
                        .checked_add(n)
                        .ok_or_else(|| io::Error::other("byte counter overflow"))?;
                    buf = &buf[n..];
                }
                Err(e) => {
                    self.error = Some(e);
                    return Err(io::Error::other("caller writer failed"));
                }
            }
        }
        Ok(initial)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_export_fixed_width_numbers_and_interrupts_are_bounded() {
        let mut bytes = Vec::new();
        let mut json = Json::new(&mut bytes);
        json.raw("[").unwrap();
        let values = [
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            -f64::MAX,
            -0.0,
        ];
        for (i, value) in values.into_iter().enumerate() {
            if i > 0 {
                json.raw(",").unwrap();
            }
            json.f64(value).unwrap();
        }
        json.raw(",").unwrap();
        json.u128(u128::MAX).unwrap();
        json.raw("]").unwrap();
        let count = json.finish().unwrap().0;
        assert_eq!(count, bytes.len());
        assert!(std::str::from_utf8(&bytes)
            .unwrap()
            .contains("340282366920938463463374607431768211455"));
        assert!(bytes.len() < 192);
        let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for (actual, expected) in decoded.as_array().unwrap()[..values.len()]
            .iter()
            .zip(values)
        {
            assert_eq!(actual.as_f64().unwrap().to_bits(), expected.to_bits());
        }
        let mut sink = Json::new(Vec::new());
        assert!(matches!(
            sink.f64(f64::NAN),
            Err(DiagnosticExportError::InvalidReport)
        ));
        assert!(matches!(
            sink.f64(f64::INFINITY),
            Err(DiagnosticExportError::InvalidReport)
        ));

        let mut interrupted = Interrupted { calls: 0 };
        let mut json = Json::new(&mut interrupted);
        match json.raw("never written").unwrap_err() {
            DiagnosticExportError::Io { source, .. } => {
                assert_eq!(source.kind(), io::ErrorKind::Interrupted)
            }
            error => panic!("unexpected error: {error:?}"),
        }
        assert_eq!(interrupted.calls, 1);
    }

    struct Interrupted {
        calls: usize,
    }
    impl Write for Interrupted {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "injected interrupt",
            ))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
