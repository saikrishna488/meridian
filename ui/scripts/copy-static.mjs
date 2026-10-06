// Copy non-TypeScript sources (HTML, CSS) from src/ to dist/, keeping layout.
import { cpSync, mkdirSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const STATIC = new Set([".html", ".css"]);

mkdirSync(join(root, "dist"), { recursive: true });
cpSync(join(root, "src"), join(root, "dist"), {
  recursive: true,
  filter: (src) => !extname(src) || STATIC.has(extname(src)),
});
