// jsdom cannot execute ESM. Resolve the production module graph into one script.
import { build } from "esbuild";
import { fileURLToPath } from "node:url";
const bundles = new Map();
export function surfaceScript(name) {
  if (!bundles.has(name)) bundles.set(name, build({
    entryPoints: [fileURLToPath(new URL(`../dist/surfaces/${name}/${name}.js`, import.meta.url))],
    bundle: true, write: false, format: "iife", platform: "browser", logLevel: "silent",
  }).then(result => result.outputFiles[0].text));
  return bundles.get(name);
}
