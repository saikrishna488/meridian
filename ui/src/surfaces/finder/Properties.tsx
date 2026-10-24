import { Dialog } from "../../components/ui/dialog";
import { Button } from "../../components/ui/button";
import { useEffect, useState } from "react";
import { request } from "../../lib/bridge";
import { errorMessage } from "../../lib/react";
import { Icon } from "../../components/Icon";
import type { FileProperties } from "../../lib/generated/FileProperties";

export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "Unavailable";
  if (bytes < 1024) return `${bytes.toLocaleString()} ${bytes === 1 ? "byte" : "bytes"}`;
  const units = ["KB", "MB", "GB", "TB", "PB"];
  let value = bytes / 1024, index = 0;
  while (value >= 1024 && index < units.length - 1) { value /= 1024; index++; }
  return `${value.toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[index]}`;
}
export function Properties({ path, close }: { path: string; close: () => void }) {
  const [info, setInfo] = useState<FileProperties | null>(null), [error, setError] = useState(""), [revision, setRevision] = useState(0);
  useEffect(() => {
    let alive = true; setInfo(null); setError("");
    void request({ method: "files.properties", params: { path } }).then(info => { if (alive) setInfo(info); }).catch(e => { if (alive) setError(errorMessage(e)); });
    return () => { alive = false; };
  }, [path, revision]);
  const date = (time: number | null) => time === null ? "Unavailable" : new Date(time * 1000).toLocaleString();
  return <Dialog open onClose={close} scrimClassName="properties-scrim" className="properties-dialog" aria-labelledby="properties-title"><header><span className="properties-symbol"><Icon name="info" width="24" height="24" /></span><div><span className="properties-eyebrow">FILE DETAILS</span><h2 id="properties-title">{info?.name ?? "Properties"}</h2></div><Button data-autofocus aria-label="Close properties" onClick={close}><Icon name="close" width="17" height="17" /></Button></header>
    {!info && !error && <p className="properties-loading" role="status">Reading file details…</p>}
    {info && <dl>{[["Kind", info.kind], ["Location", info.path], ...(info.size === null ? [] : [["Size", `${formatSize(info.size)} (${info.size.toLocaleString()} bytes)`]]), ["Modified", date(info.modified)], ["Created", date(info.created)], ["Owner", info.owner], ["Group", info.group], ["Permissions", info.permissions], ...(info.link_target === null ? [] : [["Link target", info.link_target]])].map(([name, value]) => <div key={name}><dt>{name}</dt><dd>{value}</dd></div>)}</dl>}
    {error && <div className="properties-error" role="alert"><p>{error}</p><Button onClick={() => setRevision(n => n + 1)}>Try again</Button></div>}
    <footer><span>Details are read-only.</span><Button onClick={close}>Done</Button></footer>
  </Dialog>;
}
