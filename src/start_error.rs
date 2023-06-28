use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum StartError {
    MutexPoisoned,
    OnlyOneInstanceAllowed,
    PathContainsNullByte(String),
    PathIsNotValidUtf8(PathBuf),
    DurationIsTooLong(Duration),
    Lassie(String),
    AccessTokenContainsNullByte(String),
}

impl Display for StartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to start Lassie daemon: ")?;
        match self {
            StartError::MutexPoisoned => f.write_str("the global mutex was poisoned"),
            StartError::OnlyOneInstanceAllowed => {
                f.write_str("cannot create more than one instance")
            }
            StartError::PathContainsNullByte(path_str) => f.write_fmt(format_args!(
                "null bytes are not allowed in paths (value: {path_str:?})",
            )),
            StartError::PathIsNotValidUtf8(path) => f.write_fmt(format_args!(
                "paths that are not valid UTF-8 are not supported (value: {:?})",
                path.display(),
            )),
            StartError::Lassie(msg) => f.write_str(msg),
            StartError::DurationIsTooLong(d) => f.write_fmt(format_args!(
                "duration {d:#?} is too long, Go limits the largest representable duration to approximately 290 years",
            )),
            StartError::AccessTokenContainsNullByte(token) => f.write_fmt(format_args!(
                "null bytes are not allowed in the access token (value: {token:?})",
            )),
        }
    }
}

impl std::error::Error for StartError {}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Context;
    use pretty_assertions::assert_eq;

    #[test]
    fn can_be_converted_to_anyhow_error() {
        let result: Result<(), StartError> = Err(StartError::OnlyOneInstanceAllowed);
        let anyhow = result.context("lassie error");
        // the test passes when the compiler does not complain about the line above
        // to double check, we are also asserting on the string representation
        assert_eq!(
            format!("{:#}", anyhow.unwrap_err()),
            format!("lassie error: {}", StartError::OnlyOneInstanceAllowed)
        );
    }
}
