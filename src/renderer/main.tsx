import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

// Fail fast if the app root is missing (broken HTML shell).
const container = document.getElementById("root");
if (!container) {
  throw new Error("Root element not found");
}

const root = createRoot(container);
root.render(<App />);
