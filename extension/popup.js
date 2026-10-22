const clipperCore = globalThis.MicelyaClipper;

const statusBar = document.getElementById("statusBar");
const titleInput = document.getElementById("titleInput");
const urlPreview = document.getElementById("urlPreview");
const contentPreview = document.getElementById("contentPreview");
const clipBtn = document.getElementById("clipBtn");
const shortcutHint = document.getElementById("shortcutHint");

let extractedContent = "";
let pageUrl = "";
let connected = false;

function setStatus(kind, text, detail = "") {
  statusBar.className = `status ${kind}`;
  statusBar.textContent = text;
  statusBar.title = detail;
}

async function checkConnection() {
  try {
    await clipperCore.checkServer();
    setStatus("connected", "✓ Micelya está abierta");
    return true;
  } catch (err) {
    setStatus("disconnected", "✗ Micelya no está abierta", err?.message || "");
    return false;
  }
}

async function extractContent() {
  try {
    const page = await clipperCore.extractActiveTab();
    pageUrl = page.url;
    titleInput.value = page.title;
    urlPreview.textContent = pageUrl;
    extractedContent = page.content;
    contentPreview.textContent = page.excerpt
      ? `${page.excerpt}\n\n---\n\n${extractedContent}`
      : extractedContent;
    return true;
  } catch (err) {
    contentPreview.textContent = `No se pudo leer la página: ${err.message}`;
    return false;
  }
}

async function sendClip() {
  clipBtn.disabled = true;
  setStatus("sending", "Guardando en Micelya…");
  try {
    const saved = await clipperCore.submitClip({
      title: titleInput.value,
      url: pageUrl,
      content: extractedContent,
    });
    setStatus("success", `✓ Guardada en «Recortes web»`, saved.file || "");
    clipBtn.textContent = "✓ Guardada";
  } catch (err) {
    setStatus("error", `✗ No se guardó: ${err.message}`);
    clipBtn.disabled = false;
  }
}

clipBtn.addEventListener("click", sendClip);

// Resize content preview to fill available space without causing popup scroll
function resizePreview() {
  const totalHeight = 500; // matches html/body height
  const preview = document.getElementById("contentPreview");
  if (!preview) return;
  const previewRect = preview.getBoundingClientRect();
  const bottomSpace = totalHeight - previewRect.top - 70; // button + footer
  const maxH = Math.max(100, Math.min(300, bottomSpace));
  preview.style.maxHeight = maxH + "px";
}

(async () => {
  const commands = await chrome.commands.getAll();
  const clipCommand = commands.find((command) => command.name === "clip-current-page");
  shortcutHint.textContent = clipCommand?.shortcut
    ? `Atajo: ${clipCommand.shortcut}`
    : "Podés elegir un atajo en chrome://extensions/shortcuts";
  connected = await checkConnection();
  // The page is read anyway, so it can be seen even if Micelya is closed.
  const extracted = await extractContent();
  if (!connected) {
    clipBtn.textContent = "Abrí Micelya para guardar";
  } else if (extracted) {
    clipBtn.disabled = false;
  }
  setTimeout(resizePreview, 100);
})();
