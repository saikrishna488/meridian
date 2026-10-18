import { useEffect, useRef, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage, useDialogFocus } from "../../lib/react";
import type { FilePreview } from "../../lib/generated/FilePreview";
export function Preview({ path, close }: { path: string; close: () => void }) {
  const [preview, setPreview] = useState<FilePreview | null>(null), [error, setError] = useState("");
  const dialog = useRef<HTMLElement>(null);
  useDialogFocus(dialog, true, close);
  useEffect(() => { let alive = true; void request({ method: "files.preview", params: { path } }).then(value => { if (alive) setPreview(value); }).catch(e => { if (alive) setError(errorMessage(e)); }); return () => { alive = false; }; }, [path]);
  return <div className="properties-scrim"><section className="file-preview" ref={dialog} role="dialog" aria-modal="true" aria-labelledby="preview-title"><header><h2 id="preview-title">{path.split('/').at(-1)}</h2><button data-autofocus aria-label="Close preview" onClick={close}>Close</button></header>{error ? <p role="alert">{error}</p> : !preview ? <p role="status">Loading preview…</p> : preview.kind === "text" ? <><pre tabIndex={0} aria-label="File contents">{preview.content || "Empty file"}</pre>{preview.truncated && <p role="status">Showing the first 64 KiB.</p>}</> : preview.kind === "image" ? <img src={preview.content} alt={`Preview of ${path.split('/').at(-1)}`} /> : <p>A preview is unavailable for this file. Open it in its application.</p>}<footer><button onClick={() => void request({ method: "files.open", params: { path } }).then(close).catch(e => setError(errorMessage(e)))}>Open in application</button></footer></section></div>;
}
