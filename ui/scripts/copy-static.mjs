// Copy non-TypeScript sources (HTML, CSS) from src/ to dist/, keeping layout.
import { cpSync, mkdirSync, watch } from "node:fs";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const STATIC = new Set([".html", ".css"]);

mkdirSync(join(root, "dist"), { recursive: true });
cpSync(join(root, "src"), join(root, "dist"), {
  recursive: true,
  filter: (src) => !extname(src) || STATIC.has(extname(src)) || src.endsWith("vendor/README.md"),
});

// Native WebViews still load files from dist; CSS/HTML edits need copying too.
if (process.argv.includes("--watch")) {
  let timer;
  watch(join(root, "src"), { recursive: true }, (_event, filename) => {
    if (filename && !STATIC.has(extname(filename))) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      cpSync(join(root, "src"), join(root, "dist"), {
        recursive: true,
        filter: (src) => !extname(src) || STATIC.has(extname(src)) || src.endsWith("vendor/README.md"),
      });
    }, 50);
  });
}
