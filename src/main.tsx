import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app/App";
import { LayoutProvider } from "./app/providers/LayoutProvider";
import { ThemeProvider } from "./app/providers/ThemeProvider";
import "./i18n";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <LayoutProvider>
        <App />
      </LayoutProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
