// Read-only visual preview. Serves built UI with a deterministic, in-memory host.
// Usage: npm run build --prefix ui && node tools/preview-ui.mjs
import http from "node:http";
import { readFile } from "node:fs/promises";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".png": "image/png" };
const port = Number(process.env.MERIDIAN_PREVIEW_PORT || 4173);
http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://127.0.0.1:${port}`);
    let path = decodeURIComponent(url.pathname);
    if (path === "/") { res.writeHead(302, { Location: "/ui/surfaces/settings/index.html" }); return res.end(); }
    if (path === "/preview-bridge.js") path = "/tools/preview/bridge.js";
    if (path.startsWith("/app-icons/")) {
      const id = path.slice(11);
      const names = { "meridian-pulse.desktop": "pulse", "meridian-terminal.desktop": "terminal", "meridian-finder.desktop": "finder", "meridian-settings.desktop": "settings", "firefox.desktop": "browser", "code.desktop": "code", "calendar.desktop": "calendar", "mail.desktop": "mail" };
      path = `/assets/desktop-icons/meridian/${names[id] || "generic"}.svg`;
    }
    if (path.startsWith("/icons/")) path = "/assets/desktop-icons/meridian/generic.svg";
    if (path.startsWith("/ui/desktop-icons/")) path = path.replace("/ui/", "/assets/");
    const prefix = ["/ui/", "/assets/", "/tools/preview/"].find(prefix => path.startsWith(prefix));
    if (!prefix) { res.writeHead(403); return res.end(); }
    const base = resolve(root, prefix === "/ui/" ? "ui/dist" : prefix.slice(1));
    const local = resolve(base, path.slice(prefix.length));
    if (!local.startsWith(base + sep)) { res.writeHead(403); return res.end(); }
    let data = await readFile(local);
    const extension = extname(local);
    if ([".html", ".css", ".js"].includes(extension)) {
      data = data.toString().replaceAll("meridian://", "/");
      if (extension === ".html") data = data.replace(/<meta\s+http-equiv="Content-Security-Policy"[\s\S]*?\/>/gi, "").replace("<head>", '<head><script src="/preview-bridge.js"></script>');
    }
    res.writeHead(200, { "Content-Type": types[extension] || "application/octet-stream", "Cache-Control": "no-store" });
    res.end(data);
  } catch { res.writeHead(404); res.end("Build the UI first: npm run build --prefix ui"); }
}).listen(port, "127.0.0.1", () => console.log(`Read-only UI preview: http://127.0.0.1:${port}`));
