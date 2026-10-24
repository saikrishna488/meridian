import { useEffect } from "react";
import { hello } from "../../lib/bridge";
import { mount } from "../../lib/react";
import { Dock } from "../../components/Dock";
function Panel() {
  useEffect(() => { void hello().catch(error => console.error("meridian panel:", error)); }, []);
  return <footer className="taskbar" aria-label="Desktop panel"><Dock /></footer>;
}
mount(<Panel />);
