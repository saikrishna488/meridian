import { useEffect, useState } from "react";
import { onPreferenceSave } from "../../lib/bridge";
export function SaveStatus() {
  const [text, setText] = useState("");
  useEffect(() => {
    let pending = 0, failed = false, restart = false, applied = true;
    return onPreferenceSave(event => {
      if (event.state === "saving") {
        if (!pending) { failed = false; restart = false; applied = true; }
        pending++;
      } else {
        pending = Math.max(0, pending - 1);
        failed ||= event.state === "failed";
        restart ||= event.restart; applied &&= event.applied;
      }
      setText(pending ? "Saving…" : failed ? "Couldn’t save. Review the error below." : restart ? "Saved · Sign out and back in to apply" : applied ? "Applied" : "Saved");
    });
  }, []);
  return <p className="settings-save-status" role="status" aria-live="polite">{text}</p>;
}
