// Share common modules while preserving each meridian:// surface entry path.
import "./copy-static.mjs";
import { context, build } from "esbuild";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFile } from "node:fs/promises";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const watch = process.argv.includes("--watch");
const surfaces = ["panel", "topbar", "applications", "options", "greeter", "locker", "wallpaper", "desktop-menu", "settings", "pulse", "notifications", "utilities"];
const options = {
  absWorkingDir: root,
  entryPoints: surfaces.map(name => `src/surfaces/${name}/${name}.tsx`),
  outbase: "src",
  outdir: "dist",
  bundle: true,
  splitting: true,
  chunkNames: "chunks/[name]-[hash]",
  metafile: true,
  format: "esm",
  platform: "browser",
  target: "es2022",
  jsx: "automatic",
  sourcemap: true,
  minify: !watch,
  define: { "process.env.NODE_ENV": JSON.stringify(watch ? "development" : "production") },
  plugins: [{ name: "surface-manifest", setup(builder) {
    builder.onEnd(async result => {
      if (result.errors.length) return;
      const outputs = result.metafile.outputs;
      const dependencies = (path, seen = new Set()) => {
        if (seen.has(path)) return seen;
        seen.add(path);
        for (const item of outputs[path].imports) {
          if (!item.external) dependencies(item.path, seen);
        }
        return seen;
      };
      const manifest = Object.fromEntries(Object.entries(outputs)
        .filter(([path, output]) => output.entryPoint && path.endsWith(".js"))
        .map(([path]) => [path.replace(/^dist\//, ""), [...dependencies(path)].map(item => item.replace(/^dist\//, ""))]));
      await Promise.all([
        writeFile(join(root, "dist/bundle-meta.json"), JSON.stringify(result.metafile, null, 2)),
        writeFile(join(root, "dist/surface-manifest.json"), JSON.stringify(manifest, null, 2)),
      ]);
    });
  } }],
  logLevel: "info",
};
if (watch) {
  const builder = await context(options);
  await builder.watch();
} else {
  await build(options);
}
