// Bundle each WebView independently while preserving meridian:// entry paths.
import "./copy-static.mjs";
import { context, build } from "esbuild";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const watch = process.argv.includes("--watch");
const surfaces = ["panel", "applications", "options", "greeter", "locker", "wallpaper", "desktop-menu", "settings", "finder", "terminal"];
const options = {
  absWorkingDir: root,
  entryPoints: surfaces.map(name => `src/surfaces/${name}/${name}.tsx`),
  outbase: "src",
  outdir: "dist",
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "es2022",
  jsx: "automatic",
  sourcemap: true,
  minify: !watch,
  define: { "process.env.NODE_ENV": JSON.stringify(watch ? "development" : "production") },
  logLevel: "info",
};
if (watch) {
  const builder = await context(options);
  await builder.watch();
} else {
  await build(options);
}
