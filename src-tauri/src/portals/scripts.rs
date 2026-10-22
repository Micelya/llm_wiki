//! The app's own small scripts, evaluated in the portal's view.
//!
//! They use no library and open no channel to the app: each one is
//! evaluated with `Webview::eval_with_callback` (WebView2's `ExecuteScript`),
//! which hands its last value back to the app as JSON. The page cannot
//! call the app; only the app asks the page.
//!
//! - `INIT`, on every page of the view before its own scripts: the lock.
//!   While locked, a layer over the page and a listener on the whole
//!   window stop every click and key of a person (the events a person
//!   makes are `isTrusted`; the app's own clicks are not, and pass).
//! - `snapshot`: the page's HTML. To act, it first marks the elements the
//!   app may press or fill (`data-micelya`); to only look (while a person
//!   solves the challenge) it marks nothing on the page, only on a copy.
//! - `click`, `fill`, `download`: on the element with a mark.

use super::page::{HIDDEN, MARK, VALUE};

/// The lock, installed on every page of the view. It starts locked.
pub const INIT: &str = r#"(function () {
  if (window.__micelya) return;
  var locked = true;
  var note = 'Micelya está usando esta ventana';
  var stop = function (e) {
    if (locked && e.isTrusted) { e.stopImmediatePropagation(); e.preventDefault(); }
  };
  ['click', 'dblclick', 'auxclick', 'mousedown', 'mouseup', 'pointerdown', 'pointerup', 'touchstart',
   'contextmenu', 'keydown', 'keypress', 'keyup', 'beforeinput', 'paste', 'drop'].forEach(function (t) {
    window.addEventListener(t, stop, true);
  });
  var render = function () {
    if (!document.body) return;
    var layer = document.getElementById('micelya-lock');
    if (!layer) {
      layer = document.createElement('div');
      layer.id = 'micelya-lock';
      layer.setAttribute('style', 'position:fixed;inset:0;z-index:2147483647;cursor:not-allowed;' +
        'background:rgba(15,23,42,0.06);display:flex;align-items:flex-start;justify-content:center;');
      var tag = document.createElement('div');
      tag.id = 'micelya-lock-note';
      tag.setAttribute('style', 'margin-top:8px;padding:6px 12px;border-radius:6px;background:#1e293b;color:#fff;' +
        'font:13px system-ui,sans-serif;box-shadow:0 2px 6px rgba(0,0,0,.25);');
      layer.appendChild(tag);
      document.documentElement.appendChild(layer);
    }
    layer.style.display = locked ? 'flex' : 'none';
    var tagEl = document.getElementById('micelya-lock-note');
    if (tagEl) tagEl.textContent = note;
  };
  window.__micelya = {
    lock: function (value, text) { locked = !!value; if (text) note = text; render(); return locked; }
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', render);
  else render();
})();"#;

fn js_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// The page: its address, its state and its HTML. `act`: mark the
/// elements the app may press or fill.
pub fn snapshot(act: bool) -> String {
    format!(
        r#"(function (act) {{
  try {{
    var root = document.documentElement;
    if (!root) return {{ url: location.href, ready: document.readyState, html: '' }};
    if (act) {{
      var gen = (window.__micelyaGen = (window.__micelyaGen || 0) + 1);
      var marked = root.querySelectorAll('a, button, input, select, textarea, [onclick], [role="button"]');
      for (var i = 0; i < marked.length; i++) marked[i].setAttribute('{MARK}', gen + '-' + i);
    }}
    var copy = root.cloneNode(true);
    var live = root.getElementsByTagName('*');
    var copied = copy.getElementsByTagName('*');
    for (var j = 0; j < live.length && j < copied.length; j++) {{
      var el = live[j];
      var hidden = el.getClientRects().length === 0;
      if (!hidden) {{ var style = window.getComputedStyle(el); hidden = style.visibility === 'hidden'; }}
      if (hidden) copied[j].setAttribute('{HIDDEN}', '1');
      if (el.tagName === 'INPUT' || el.tagName === 'SELECT' || el.tagName === 'TEXTAREA') {{
        if (el.type !== 'password') copied[j].setAttribute('{VALUE}', el.value || '');
      }}
    }}
    var lock = copy.querySelector('#micelya-lock');
    if (lock) lock.parentNode.removeChild(lock);
    return {{ url: location.href, ready: document.readyState, html: copy.outerHTML }};
  }} catch (e) {{
    return {{ url: location.href, error: String(e) }};
  }}
}})({act})"#
    )
}

/// Presses the element with this mark. Answers "ok" or "missing".
pub fn click(mark: &str) -> String {
    format!(
        r#"(function (m) {{
  var el = document.querySelector('[{MARK}="' + m + '"]');
  if (!el) return 'missing';
  el.click();
  return 'ok';
}})({})"#,
        js_string(mark)
    )
}

/// Puts a value in the field with this mark (for a list, the option with
/// that value), as a person would. Answers "ok", "missing" or "no-option".
pub fn fill(mark: &str, value: &str) -> String {
    format!(
        r#"(function (m, v) {{
  var el = document.querySelector('[{MARK}="' + m + '"]');
  if (!el) return 'missing';
  if (el.tagName === 'SELECT') {{
    var found = -1;
    for (var i = 0; i < el.options.length; i++) {{
      var o = el.options[i];
      if (o.value === v || o.text.replace(/\s+/g, ' ').trim() === v) {{ found = i; break; }}
    }}
    if (found < 0) return 'no-option';
    el.selectedIndex = found;
  }} else {{
    el.focus();
    el.value = v;
  }}
  el.dispatchEvent(new Event('input', {{ bubbles: true }}));
  el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  if (el.blur) el.blur();
  return 'ok';
}})({}, {})"#,
        js_string(mark),
        js_string(value)
    )
}

/// Asks for the document of the link with this mark. A link with an
/// address is downloaded as a download (never opened in the view); one
/// without (a button of the form) is pressed. Answers "link", "click" or
/// "missing".
pub fn download(mark: &str) -> String {
    format!(
        r#"(function (m) {{
  var el = document.querySelector('[{MARK}="' + m + '"]');
  if (!el) return 'missing';
  var href = el.getAttribute('href');
  if (href && href !== '#' && href.indexOf('javascript:') !== 0) {{
    var a = document.createElement('a');
    a.href = el.href;
    a.setAttribute('download', '');
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    a.parentNode.removeChild(a);
    return 'link';
  }}
  el.click();
  return 'click';
}})({})"#,
        js_string(mark)
    )
}

/// Locks or unlocks the view, with the note it shows while locked.
pub fn lock(locked: bool, note: &str) -> String {
    format!("(window.__micelya ? window.__micelya.lock({locked}, {}) : null)", js_string(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_go_into_the_scripts_as_javascript_strings() {
        let script = fill("3-7", "CIV ' \" </script>");
        assert!(script.contains(r#""CIV ' \" </script>""#));
        assert!(click("1-2").contains("\"1-2\""));
        assert!(snapshot(true).contains("})(true)"));
        assert!(snapshot(false).contains("})(false)"));
        assert!(lock(false, "x").contains("lock(false, \"x\")"));
    }

    #[test]
    fn the_lock_only_stops_what_a_person_does() {
        assert!(INIT.contains("e.isTrusted"));
        assert!(INIT.contains("var locked = true"));
    }
}
