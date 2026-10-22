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
//!   Only on the page itself, never inside a frame of it (see `INIT`).
//! - `snapshot`: the page's HTML. To act, it first marks the elements the
//!   app may press or fill (`data-micelya`); to only look (while a person
//!   solves the challenge) it marks nothing on the page, only on a copy.
//! - `click`, `fill`, `download`: on the element with a mark.
//! - `focus`: while the person solves the portal's challenge, the page is
//!   taken to its box and everything else is darkened (`FOCUS`).

use super::page::{HIDDEN, MARK, VALUE};

/// The lock, installed on every page of the view. It starts locked.
///
/// - Only on the page itself. On Windows the view runs it in every frame
///   too (Tauri: "scripts are always added to subframes"), and the app
///   unlocks only the page (`lock` is evaluated in the page, never in its
///   frames): a frame that locked itself, like the portal's challenge, a
///   frame of another site, would stay locked for good. While the page is
///   locked its layer covers its frames too, so a click on a frame lands
///   on the layer.
/// - Locking takes the focus away from where it is (a field, or a frame:
///   the keys of a frame do not pass by the page's listener), so the
///   person cannot go on typing there while the app works.
pub const INIT: &str = r#"(function () {
  if (window.top !== window) return;
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
  var unfocus = function () {
    var active = document.activeElement;
    if (active && active !== document.body && typeof active.blur === 'function') active.blur();
  };
  window.__micelya = {
    lock: function (value, text) { locked = !!value; if (text) note = text; if (locked) unfocus(); render(); return locked; }
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
    ['#micelya-lock', '#micelya-focus'].forEach(function (own) {{
      var layer = copy.querySelector(own);
      if (layer) layer.parentNode.removeChild(layer);
    }});
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

/// The focus on the portal's challenge, a function of `on`, evaluated in
/// the page (never in its frames).
///
/// - Its box is the challenge's frame (an `iframe` that names `captcha` in
///   its id, name, class or address, as `captcha-frame` of the PJN), or
///   else the «Ver desafío» button.
/// - On: the page is taken to the box once, and a layer of the app's own
///   (not of the portal) darkens everything else: one shade cut by an
///   exact hole over the box (an even-odd `clip-path`, which also leaves
///   the hole out of its clicks), following its size and place on every
///   frame. The shade takes the clicks outside the hole and does nothing
///   with them (the wheel still scrolls the page); inside the hole nothing
///   of the app is in the way. A note over the hole says what to do, and
///   lets the person take the layer away for this page.
/// - It goes by itself when the challenge is solved (the form's
///   `captcha-response` field has a value), when the box goes, or with the
///   page. It never changes the challenge nor the form; it only reads the
///   value of that field. Under the lock's layer (`INIT`).
/// - Answers `on`, `off`, `none` (no box), `solved` or `dismissed`.
pub const FOCUS: &str = r#"function (on) {
  var ID = 'micelya-focus';
  var state = window.__micelyaFocus || (window.__micelyaFocus = { frame: 0, dismissed: false });
  var remove = function () {
    if (state.frame) cancelAnimationFrame(state.frame);
    state.frame = 0;
    var old = document.getElementById(ID);
    if (old && old.parentNode) old.parentNode.removeChild(old);
  };
  var solved = function () {
    var fields = document.querySelectorAll('[name="captcha-response"], [id="captcha-response"], ' +
      '[name$=":captcha-response"], [id$=":captcha-response"]');
    for (var i = 0; i < fields.length; i++) if (fields[i].value) return true;
    return false;
  };
  var shown = function (el) { return el.getClientRects().length > 0; };
  var findBox = function () {
    var frames = document.getElementsByTagName('iframe');
    for (var i = 0; i < frames.length; i++) {
      var f = frames[i];
      var names = [f.id, f.name, f.className, f.getAttribute('src')].join(' ').toLowerCase();
      if (names.indexOf('captcha') >= 0 && shown(f)) return f;
    }
    var buttons = document.querySelectorAll('button, a, input[type="button"], input[type="submit"]');
    for (var j = 0; j < buttons.length; j++) {
      var b = buttons[j];
      var words = (b.tagName === 'INPUT' ? b.value : b.textContent || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (words.normalize) words = words.normalize('NFD').replace(/[̀-ͯ]/g, '');
      if (words.indexOf('ver desafio') === 0 && words.length <= 40 && shown(b)) return b;
    }
    return null;
  };
  if (!on) { state.dismissed = false; remove(); return 'off'; }
  if (state.dismissed) { remove(); return 'dismissed'; }
  if (solved()) { remove(); return 'solved'; }
  if (document.getElementById(ID)) return 'on';
  var box = findBox();
  if (!box || !document.body) return 'none';
  var swallow = function (el) {
    ['click', 'dblclick', 'auxclick', 'mousedown', 'mouseup', 'pointerdown', 'pointerup', 'touchstart', 'contextmenu']
      .forEach(function (t) { el.addEventListener(t, function (e) { e.stopPropagation(); e.preventDefault(); }); });
  };
  var layer = document.createElement('div');
  layer.id = ID;
  layer.setAttribute('style', 'position:fixed;inset:0;z-index:2147483646;pointer-events:none;');
  var shade = document.createElement('div');
  swallow(shade);
  layer.appendChild(shade);
  var ring = document.createElement('div');
  layer.appendChild(ring);
  var note = document.createElement('div');
  note.setAttribute('style', 'position:fixed;pointer-events:auto;max-width:340px;padding:8px 12px;border-radius:6px;' +
    'background:#1e293b;color:#fff;font:13px/1.35 system-ui,sans-serif;box-shadow:0 2px 8px rgba(0,0,0,.35);');
  var text = document.createElement('div');
  text.textContent = 'Resolvé el desafío en este recuadro. Cuando lo resuelvas, tocá «Consultar».';
  var away = document.createElement('button');
  away.type = 'button';
  away.textContent = 'Quitar el oscurecido';
  away.setAttribute('style', 'margin-top:6px;padding:2px 8px;border:1px solid #94a3b8;border-radius:4px;' +
    'background:transparent;color:#e2e8f0;font:12px system-ui,sans-serif;cursor:pointer;');
  away.addEventListener('click', function () { state.dismissed = true; remove(); });
  note.appendChild(text);
  note.appendChild(away);
  swallow(note);
  layer.appendChild(note);
  document.documentElement.appendChild(layer);
  try { box.scrollIntoView({ block: 'center', inline: 'nearest' }); } catch (e) { box.scrollIntoView(); }
  var last = '';
  var place = function (x, y, w, h) {
    var hole = [[x, y], [x, y + h], [x + w, y + h], [x + w, y], [x, y]].map(function (p) {
      return p[0] + 'px ' + p[1] + 'px';
    }).join(',');
    shade.setAttribute('style', 'position:fixed;inset:0;background:rgba(15,23,42,0.62);pointer-events:auto;' +
      'clip-path:polygon(evenodd,0 0,100% 0,100% 100%,0 100%,0 0,' + hole + ');');
    ring.setAttribute('style', 'position:fixed;box-sizing:border-box;pointer-events:none;border:3px solid #f59e0b;' +
      'border-radius:6px;left:' + x + 'px;top:' + y + 'px;width:' + w + 'px;height:' + h + 'px;');
    var width = document.documentElement.clientWidth;
    var above = y - note.offsetHeight - 10;
    var top = above >= 4 ? above : y + h + 10;
    var left = Math.max(8, Math.min(x, width - note.offsetWidth - 8));
    note.style.top = top + 'px';
    note.style.left = left + 'px';
  };
  var tick = function () {
    state.frame = 0;
    if (!document.getElementById(ID)) return;
    if (!box.isConnected || !shown(box) || solved()) { remove(); return; }
    var r = box.getBoundingClientRect();
    var pad = 6;
    var x = Math.round(r.left - pad), y = Math.round(r.top - pad);
    var w = Math.round(r.width + 2 * pad), h = Math.round(r.height + 2 * pad);
    var key = [x, y, w, h, window.innerWidth, window.innerHeight].join(',');
    if (key !== last) { last = key; place(x, y, w, h); }
    state.frame = requestAnimationFrame(tick);
  };
  tick();
  return 'on';
}"#;

/// Turns the focus on the challenge on or off (`FOCUS`).
pub fn focus(on: bool) -> String {
    format!("({FOCUS})({on})")
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

    #[test]
    fn the_lock_stays_out_of_the_frames_of_the_page() {
        // Windows runs it in every frame too, and the app unlocks only the
        // page: a frame must leave before it locks anything.
        let guard = INIT.find("if (window.top !== window) return;").expect("the lock checks it is the page itself");
        for later in ["var locked = true", "window.addEventListener(t, stop, true)", "document.createElement('div')", "window.__micelya = {"] {
            let at = INIT.find(later).unwrap_or_else(|| panic!("`{later}` is not in the lock"));
            assert!(guard < at, "the lock does `{later}` before it checks it is the page itself");
        }
    }

    #[test]
    fn locking_takes_the_focus_away() {
        assert!(INIT.contains("if (locked) unfocus();"));
        assert!(INIT.contains("active.blur()"));
    }

    #[test]
    fn the_focus_on_the_challenge_is_a_layer_of_its_own_under_the_lock() {
        assert!(focus(true).ends_with("})(true)"));
        assert!(focus(false).ends_with("})(false)"));
        // Under the lock's layer, and the layer itself takes no click: only
        // its shade, out of the hole, does, and does nothing with them.
        assert!(INIT.contains("z-index:2147483647"));
        assert!(FOCUS.contains("z-index:2147483646;pointer-events:none;"));
        assert!(FOCUS.contains("clip-path:polygon(evenodd,"));
        assert!(FOCUS.contains("e.stopPropagation(); e.preventDefault();"));
        assert!(!FOCUS.contains("'wheel'"), "the wheel still scrolls the page");
        // It follows the box on every frame, and goes once it is solved.
        assert!(FOCUS.contains("requestAnimationFrame(tick)"));
        assert!(FOCUS.contains("captcha-response"));
        // It never touches the challenge nor the form: it adds its own layer
        // and only reads.
        for touching in [".value =", ".click()", ".submit(", "setAttribute('value'", "postMessage", ".focus()"] {
            assert!(!FOCUS.contains(touching), "the focus does `{touching}`");
        }
        // The page is read without the app's own layers.
        assert!(snapshot(false).contains("'#micelya-lock', '#micelya-focus'"));
    }
}
