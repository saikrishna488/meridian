// Run after npm run build. Count unique production assets, excluding source maps.
import { readFile, readdir, stat } from 'node:fs/promises';
const dist = new URL('../dist/', import.meta.url);
const meta = JSON.parse(await readFile(new URL('bundle-meta.json', dist), 'utf8'));
const outputs = meta.outputs;
function dependencies(path, seen = new Set()) {
  if (seen.has(path)) return seen;
  seen.add(path);
  for (const item of outputs[path].imports) if (!item.external) dependencies(item.path, seen);
  return seen;
}
async function cssBytes(directory) {
  let bytes = 0;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const file = new URL(entry.name + (entry.isDirectory() ? '/' : ''), directory);
    if (entry.isDirectory()) bytes += await cssBytes(file);
    else if (entry.name.endsWith('.css')) bytes += (await stat(file)).size;
  }
  return bytes;
}
console.log(JSON.stringify({
  method: 'production esbuild bytes; unique JS files exclude maps; per surface transitive static graph',
  total_js: Object.entries(outputs).filter(([path]) => path.endsWith('.js')).reduce((sum, [, output]) => sum + output.bytes, 0),
  total_css: await cssBytes(dist),
  surfaces: Object.fromEntries(Object.entries(outputs).filter(([path, output]) => path.endsWith('.js') && output.entryPoint)
    .map(([path]) => [path, [...dependencies(path)].reduce((sum, item) => sum + outputs[item].bytes, 0)])),
}, null, 2));
