//! Authentication for the remote restart endpoints (`/restart`, `/api/restart`).
//!
//! The token is baked in at build time from the `RESTART_TOKEN` environment
//! variable, which `build.rs` reads out of the gitignored `wifi_config.h`
//! alongside the WiFi credentials (or straight from the process environment).
//!
//! There is deliberately **no default value**. Until this module existed the
//! token was a compile-time constant, `"esp32-restart"`, written into the
//! handlers, the dashboard JavaScript and the templates -- so every device
//! flashed from this public repository shipped the same restart credential,
//! readable by anyone who could open the repo. A build with no token
//! provisioned now disables the restart endpoints outright; falling back to a
//! shared default is precisely the failure mode this replaces.

/// The token provisioned at build time, or `None` if the build had none.
///
/// When nothing is provisioned `build.rs` emits no `cargo:rustc-env` directive
/// at all, so `option_env!` resolves to `None`. The empty-string arm covers the
/// other route in: a variable exported as `RESTART_TOKEN=` in the build
/// environment, which must not authorise a request presenting an empty header.
pub fn restart_token() -> Option<&'static str> {
    match option_env!("RESTART_TOKEN") {
        Some(token) if !token.is_empty() => Some(token),
        _ => None,
    }
}

/// Whether the presented `X-Restart-Token` header matches the provisioned token.
///
/// Returns `false` when no token is provisioned, so an unprovisioned build
/// rejects every request rather than accepting any.
///
/// The comparison runs over the whole token rather than stopping at the first
/// differing byte. `==` on `&str` is free to short-circuit, which would let a
/// caller on the same network recover the token a byte at a time by timing the
/// rejections.
pub fn token_matches(presented: &str) -> bool {
    match restart_token() {
        Some(expected) => constant_time_eq(presented.as_bytes(), expected.as_bytes()),
        None => false,
    }
}

/// The OTA upload password provisioned at build time, or `None` if absent.
///
/// Same contract as [`restart_token`], and the same history: `/ota/update` was
/// guarded by a four-letter hardcoded constant in this public repository, which
/// made flashing arbitrary firmware to any reachable device a matter of reading
/// the source.
pub fn ota_password() -> Option<&'static str> {
    match option_env!("OTA_PASSWORD") {
        Some(password) if !password.is_empty() => Some(password),
        _ => None,
    }
}

/// Whether the presented `X-OTA-Password` header matches the provisioned value.
///
/// Returns `false` when none is provisioned, closing the endpoint rather than
/// opening it.
pub fn ota_password_matches(presented: &str) -> bool {
    match ota_password() {
        Some(expected) => constant_time_eq(presented.as_bytes(), expected.as_bytes()),
        None => false,
    }
}

/// Compare two byte strings without short-circuiting on the first difference.
fn constant_time_eq(presented: &[u8], expected: &[u8]) -> bool {
    // A length mismatch is decided up front (length is not secret), but the
    // byte loop below still runs to completion for equal-length inputs.
    let mut diff: u32 = if presented.len() == expected.len() { 0 } else { 1 };
    let len = core::cmp::max(presented.len(), expected.len());
    for i in 0..len {
        let a = presented.get(i).copied().unwrap_or(0) as u32;
        let b = expected.get(i).copied().unwrap_or(0) as u32;
        diff |= a ^ b;
    }
    diff == 0
}

/// Escape a value for embedding inside a single-quoted JavaScript string
/// literal that itself sits inside an HTML `<script>` block.
///
/// `<` and `/` are escaped as well as the usual string metacharacters, because
/// an unescaped `</script>` inside a string literal still terminates the block.
fn escape_for_js(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '/' => out.push_str("\\/"),
            _ => out.push(c),
        }
    }
    out
}

/// The token escaped for embedding inside a single-quoted JavaScript string.
///
/// Returns an empty string when unprovisioned, which makes the dashboard's
/// restart button fail closed against the 503 the handlers return.
pub fn token_for_js() -> String {
    escape_for_js(restart_token().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_header_never_matches() {
        // Holds whether or not this build provisioned a token: with none the
        // guard clause rejects, with one the length check rejects.
        assert!(!token_matches(""));
    }

    #[test]
    fn wrong_token_never_matches() {
        assert!(!token_matches("definitely-not-the-token"));
    }

    #[test]
    fn the_old_hardcoded_token_is_not_accepted_by_default() {
        // Regression guard for the vulnerability this module replaced: a build
        // that does not explicitly provision this value must reject it.
        if restart_token() != Some("esp32-restart") {
            assert!(!token_matches("esp32-restart"));
        }
    }

    #[test]
    fn provisioned_token_matches_itself() {
        if let Some(token) = restart_token() {
            assert!(token_matches(token));
        }
    }

    #[test]
    fn js_escaping_cannot_terminate_the_script_block() {
        // Both `<` and `/` come back as escape sequences, so no raw closing
        // tag survives to end the <script> block early.
        assert_eq!(escape_for_js("</script>"), "\\u003c\\/script>");
        assert!(!escape_for_js("a'b</script>").contains("</script>"));
    }

    #[test]
    fn js_escaping_neutralises_quote_and_backslash() {
        // A bare ' would close the literal; a trailing \ would escape the
        // closing quote. Both must come back doubled/prefixed.
        assert_eq!(escape_for_js("a'b"), "a\\'b");
        assert_eq!(escape_for_js("a\\b"), "a\\\\b");
        assert_eq!(escape_for_js("a\nb"), "a\\nb");
    }

    #[test]
    fn js_escaping_leaves_ordinary_tokens_untouched() {
        // The documented generator is `openssl rand -hex 24`, which produces
        // only [0-9a-f] — escaping must not mangle it.
        let hex = "9f2c41ab77de05b3c8a16e4d2f70bb91ce33a5d7";
        assert_eq!(escape_for_js(hex), hex);
    }
}
