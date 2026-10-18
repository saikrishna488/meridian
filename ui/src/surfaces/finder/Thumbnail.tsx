import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import type { FileEntry } from "../../lib/generated/FileEntry";
const cache = new Map<string, string | null>();
const queue: (() => void)[] = [];
const pending = new Map<string, Promise<string | null>>();
let cacheBytes = 0;
const CACHE_BYTES = 4 * 1024 * 1024;
let running = 0;
function next() { if (running < 3 && queue.length) queue.shift()!(); }
function load(file: FileEntry): Promise<string | null> {
  const key = `${file.path}:${file.modified}:${file.size}`;
  if (cache.has(key)) {
    const value = cache.get(key)!; cache.delete(key); cache.set(key, value);
    return Promise.resolve(value);
  }
  if (pending.has(key)) return pending.get(key)!;
  const result = new Promise<string | null>(resolve => {
    queue.push(() => {
      running++;
      void request({ method: "files.thumbnail", params: { path: file.path } }).catch(() => null).then(value => {
        const bytes = (value?.length ?? 0) * 2;
        if (bytes <= CACHE_BYTES) {
          while (cache.size >= 128 || cacheBytes + bytes > CACHE_BYTES) {
            const oldest = cache.keys().next().value!;
            cacheBytes -= (cache.get(oldest)?.length ?? 0) * 2; cache.delete(oldest);
          }
          cache.set(key, value); cacheBytes += bytes;
        }
        resolve(value);
      }).finally(() => { running--; next(); });
    }); next();
  }).finally(() => { pending.delete(key); });
  pending.set(key, result);
  return result;
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
