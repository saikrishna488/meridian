import { useEffect } from "react";
import { hello } from "../../lib/bridge";
import { mount } from "../../lib/react";
import { Dock } from "../../components/Dock";
function Panel() {
  useEffect(() => { void hello().catch(error => console.error("meridian dock:", error)); }, []);
  return <footer className="taskbar" aria-label="Desktop dock"><Dock /></footer>;
}
mount(<Panel />);
