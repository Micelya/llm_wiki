importScripts("clipper-core.js");

const COMMAND_NAME = "clip-current-page";
const TITLE = "Micelya Clipper";
let badgeTimer;
let clipInFlight = false;

async function setBadge(text, color, title, clearAfterMs = 0) {
  clearTimeout(badgeTimer);
  await chrome.action.setBadgeBackgroundColor({ color });
  await chrome.action.setBadgeText({ text });
  if (title) await chrome.action.setTitle({ title });
  if (clearAfterMs > 0) {
    badgeTimer = setTimeout(() => {
      void chrome.action.setBadgeText({ text: "" });
      void chrome.action.setTitle({ title: TITLE });
    }, clearAfterMs);
  }
}

async function clipCurrentPage(commandTab) {
  if (clipInFlight) {
    await setBadge("…", "#4f46e5", "Ya se está guardando una página");
    return;
  }
  clipInFlight = true;
  const core = globalThis.MicelyaClipper;
  try {
    await setBadge("…", "#4f46e5", "Guardando la página…");
    await core.checkServer();
    const page = await core.extractActiveTab(commandTab);
    const saved = await core.submitClip(page);
    await setBadge("✓", "#059669", `Guardada en «Recortes web»: ${saved.file || ""}`, 4000);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("[Micelya Clipper] no se pudo guardar:", error);
    await setBadge("!", "#dc2626", `No se guardó: ${message}`, 7000);
  } finally {
    clipInFlight = false;
  }
}

chrome.commands.onCommand.addListener((command, tab) => {
  if (command === COMMAND_NAME) void clipCurrentPage(tab);
});
