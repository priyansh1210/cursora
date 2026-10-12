import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Chat from "./Chat";
import Settings from "./Settings";
import Setup from "./Setup";
import "./styles.css";

// One bundle for every window; the window label picks the view.
const label = getCurrentWindow().label;
const View = label === "bubble" || label === "chat" ? Chat : label === "setup" ? Setup : Settings;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
