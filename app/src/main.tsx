import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

async function boot() {
  const params = new URLSearchParams(window.location.search);
  const mock = params.has("mock");
  // Mock bridge first so ?mock=1 works without the Tauri runtime.
  if (mock) {
    await import("./mock");
  }
  // The macOS overlay title bar puts the traffic lights over the webview. The
  // stylesheet pads the topbar when data-os="mac"; set it only inside the real
  // app so browser previews and screenshots keep the plain layout.
  const forced = params.get("os");
  const inTauri = Boolean((window as unknown as Record<string, any>).__TAURI_INTERNALS__);
  const isMac =
    forced === "mac" ||
    (!mock && inTauri && /Mac/i.test(navigator.platform || navigator.userAgent));
  if (isMac) {
    document.documentElement.dataset.os = "mac";
  }
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

boot();
