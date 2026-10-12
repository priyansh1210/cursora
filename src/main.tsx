import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Settings from "./Settings";
import Setup from "./Setup";
import "./styles.css";

// One bundle for every window; the window label picks the view.
const View = getCurrentWindow().label === "setup" ? Setup : Settings;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <View />
  </React.StrictMode>,
);
