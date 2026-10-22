(function initializeClipperCore(global) {
  // Micelya listens only on the machine itself.
  const SERVER_URLS = ["http://127.0.0.1:19827", "http://localhost:19827"];
  const MAX_EXTRACTED_CONTENT_CHARS = 1_000_000;
  const TRUNCATION_NOTICE = "\n\n[Micelya Clipper: el contenido de la página se cortó en 1.000.000 de caracteres.]";

  function limitExtractedContent(content) {
    const value = String(content || "");
    if (value.length <= MAX_EXTRACTED_CONTENT_CHARS) return value;
    return `${value.slice(0, MAX_EXTRACTED_CONTENT_CHARS)}${TRUNCATION_NOTICE}`;
  }

  async function clipFetch(path, options) {
    const method = String(options?.method || "GET").toUpperCase();
    // A clip is never sent twice: the first one may have arrived even if
    // its answer was lost.
    const urls = method === "GET" ? SERVER_URLS : SERVER_URLS.slice(0, 1);
    let lastError;
    for (const baseUrl of urls) {
      try {
        return await fetch(`${baseUrl}${path}`, options);
      } catch (error) {
        lastError = error;
      }
    }
    throw lastError || new Error("No se pudo conectar con Micelya");
  }

  async function checkServer() {
    const response = await clipFetch("/status", { method: "GET" });
    const data = await response.json();
    if (!response.ok || !data.ok) throw new Error(data.error || "Micelya no respondió");
    return true;
  }

  // This function is serialized into the active tab by chrome.scripting, so it
  // must remain self-contained and must not capture extension-scope variables.
  function extractReadablePage() {
    try {
      const documentClone = document.cloneNode(true);
      const reader = new window.Readability(documentClone);
      const article = reader.parse();
      if (!article || !article.content) {
        return { error: "No se pudo extraer el contenido de la página" };
      }

      const turndown = new window.TurndownService({
        headingStyle: "atx",
        codeBlockStyle: "fenced",
        bulletListMarker: "-",
      });
      turndown.addRule("tableCell", {
        filter: ["th", "td"],
        replacement: (content) => ` ${content.trim()} |`,
      });
      turndown.addRule("tableRow", {
        filter: "tr",
        replacement: (content) => `|${content}\n`,
      });
      turndown.addRule("table", {
        filter: "table",
        replacement: (content) => {
          const lines = content.trim().split("\n");
          if (lines.length > 0) {
            const columns = (lines[0].match(/\|/g) || []).length - 1;
            lines.splice(1, 0, `|${" --- |".repeat(columns)}`);
          }
          return `\n\n${lines.join("\n")}\n\n`;
        },
      });
      turndown.addRule("removeSmallImages", {
        filter: (node) => {
          if (node.nodeName !== "IMG") return false;
          const width = parseInt(node.getAttribute("width") || "999");
          const height = parseInt(node.getAttribute("height") || "999");
          return width < 10 || height < 10;
        },
        replacement: () => "",
      });

      return {
        title: article.title || document.title || "",
        content: turndown.turndown(article.content),
        excerpt: article.excerpt || "",
      };
    } catch (error) {
      return { error: error instanceof Error ? error.message : String(error) };
    }
  }

  function extractFallbackPage() {
    const clone = document.body?.cloneNode(true);
    if (!clone) return "";
    ["script", "style", "nav", "header", "footer", ".sidebar", ".ad", ".comments"]
      .forEach((selector) => clone.querySelectorAll(selector).forEach((element) => element.remove()));
    return clone.innerText
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0)
      .join("\n\n");
  }

  async function extractActiveTab(commandTab) {
    // Chrome passes the exact shortcut target to commands.onCommand together
    // with the temporary activeTab grant. Popup callers do not have that value
    // and resolve their own currently active tab instead.
    const tab = commandTab?.id
      ? commandTab
      : (await chrome.tabs.query({ active: true, currentWindow: true }))[0];
    if (!tab?.id) throw new Error("No hay una pestaña activa");
    if (!/^https?:\/\//i.test(tab.url || "")) {
      throw new Error("Esta página del navegador no se puede recortar");
    }

    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["Readability.js", "Turndown.js"],
    });
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: extractReadablePage,
    });
    const extracted = results?.[0]?.result;
    let content = extracted?.content || "";
    if (!content) {
      const fallback = await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        func: extractFallbackPage,
      });
      content = fallback?.[0]?.result || "";
    }
    if (!content.trim()) throw new Error(extracted?.error || "No se pudo extraer el contenido de la página");
    content = limitExtractedContent(content);

    return {
      title: extracted?.title || tab.title || "",
      url: tab.url || "",
      content,
      excerpt: extracted?.excerpt || "",
    };
  }

  async function submitClip(page) {
    const response = await clipFetch("/clip", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        title: page.title,
        url: page.url,
        content: page.content,
      }),
    });
    const data = await response.json();
    if (!response.ok || !data.ok) throw new Error(data.error || `Micelya no lo guardó (HTTP ${response.status})`);
    return data;
  }

  global.MicelyaClipper = Object.freeze({
    SERVER_URLS,
    MAX_EXTRACTED_CONTENT_CHARS,
    checkServer,
    extractActiveTab,
    submitClip,
  });
})(globalThis);
