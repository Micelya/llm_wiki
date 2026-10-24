//! Who may talk to the server (7 and 11.8 of the contract's design), in
//! this order: the request has to be addressed to the server by the name
//! of the machine itself (a page of a malicious site that renamed itself
//! to this machine's address is refused: «DNS rebinding»); a web page is
//! never let in, only the browser's extensions or programs, which send no
//! `Origin`; and the local API and the MCP need the token of a program,
//! compared without telling how much of it matched.

/// Whether the `Host` of a request names this server: `127.0.0.1:<port>`
/// or `localhost:<port>`, nothing else.
pub fn host_is_ours(host: Option<&str>, port: u16) -> bool {
    let Some(host) = host.map(str::trim) else { return false };
    let host = host.to_ascii_lowercase();
    host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}")
}

/// Whether an `Origin` is one of the browser's extensions (the web
/// clipper). A web page, the window of a judicial portal included, never is.
pub fn is_extension(origin: &str) -> bool {
    let rest = origin.strip_prefix("chrome-extension://").or_else(|| origin.strip_prefix("moz-extension://"));
    rest.is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// The token of an `Authorization: Bearer …` header.
pub fn bearer(header: Option<&str>) -> Option<&str> {
    let value = header?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then_some(token)
}

/// Compares without telling how much matched.
pub fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_machine_itself_by_its_own_name_is_the_host() {
        assert!(host_is_ours(Some("127.0.0.1:19827"), 19827));
        assert!(host_is_ours(Some("localhost:19827"), 19827));
        assert!(host_is_ours(Some("LocalHost:5000"), 5000));
        for host in [None, Some(""), Some("127.0.0.1"), Some("127.0.0.1:19828"), Some("sitio.invalid:19827"), Some("localhost.sitio.invalid:19827"), Some("192.168.1.20:19827")] {
            assert!(!host_is_ours(host, 19827), "{host:?}");
        }
    }

    #[test]
    fn only_the_extensions_of_the_browser_are_an_origin_let_in() {
        for origin in ["chrome-extension://abcdefghijklmnop", "moz-extension://1b2c-3d4e"] {
            assert!(is_extension(origin), "{origin}");
        }
        for origin in ["", "null", "https://sitio.invalid", "http://localhost:5173", "http://127.0.0.1:8080", "chrome-extension://", "chrome-extension://abc/../x", "http://tauri.localhost"] {
            assert!(!is_extension(origin), "{origin}");
        }
    }

    #[test]
    fn the_token_is_read_from_its_header_and_compared_whole() {
        assert_eq!(bearer(Some("Bearer abc")), Some("abc"));
        assert_eq!(bearer(Some("bearer  abc ")), Some("abc"));
        assert_eq!(bearer(Some("Basic abc")), None);
        assert_eq!(bearer(Some("Bearer ")), None);
        assert_eq!(bearer(None), None);
        assert!(same(b"abc", b"abc") && !same(b"abc", b"abd") && !same(b"abc", b"ab"));
    }
}
