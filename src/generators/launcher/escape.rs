use std::fmt::Write;

use crate::error::{KalesaError, Result};

pub fn bash_quote(value: &str) -> Result<String> {
    if value.contains('\0') {
        return Err(KalesaError::InvalidDesktopValue(
            "launcher value cannot contain NUL".into(),
        ));
    }

    let escaped = value.replace('\'', "'\\''");
    Ok(format!("'{escaped}'"))
}

pub fn push_array(out: &mut String, name: &str, values: &[String]) -> Result<()> {
    write!(out, "{name}=\(").expect("writing to String cannot fail");
    for value in values {
        write!(out, " {}", bash_quote(value)?).expect("writing to String cannot fail");
    }
    out.push_str(" )\n");
    Ok(())
}

pub fn push_env(out: &mut String, env: &std::collections::BTreeMap<String, String>) -> Result<()> {
    for (key, value) in env {
        write!(out, "export {key}={};\n", bash_quote(value)?)
            .expect("writing to String cannot fail");
    }
    Ok(())
}
