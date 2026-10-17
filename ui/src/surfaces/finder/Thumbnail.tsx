import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import type { FileEntry } from "../../lib/generated/FileEntry";
const cache = new Map<string, string | null>();
const queue: (() => void)[] = [];
let running = 0;
function next() { if (running < 3 && queue.length) queue.shift()!(); }
function load(file: FileEntry): Promise<string | null> {
  const key = `${file.path}:${file.modified}:${file.size}`;
  if (cache.has(key)) return Promise.resolve(cache.get(key)!);
  return new Promise(resolve => {
    queue.push(() => {
      running++;
      void request({ method: "files.thumbnail", params: { path: file.path } }).catch(() => null).then(value => {
        if (cache.size >= 128) cache.delete(cache.keys().next().value!);
        cache.set(key, value); resolve(value);
      }).finally(() => { running--; next(); });
    }); next();
  });
}
export function imageFile(file?: FileEntry) { return !!file && !file.directory && /\.(png|jpe?g|webp|gif|bmp)$/i.test(file.name); }
export function Thumbnail({ file, fallback }: { file: FileEntry; fallback: React.ReactNode }) {
  const ref = useRef<HTMLSpanElement>(null), [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    setSrc(null);
    const show = () => { void load(file).then(value => { if (alive) setSrc(value); }); };
    if (!ref.current || typeof IntersectionObserver === "undefined") { show(); return () => { alive = false; }; }
    const observer = new IntersectionObserver(entries => { if (entries.some(e => e.isIntersecting)) { observer.disconnect(); show(); } }, { rootMargin: "100px" });
    observer.observe(ref.current);
    return () => { alive = false; observer.disconnect(); };
  }, [file.path, file.modified, file.size]);
  return <span ref={ref} className="file-thumbnail">{src ? <img src={src} alt="" /> : fallback}</span>;
}
