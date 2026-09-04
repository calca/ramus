import React from "react";
import ReactDOM from "react-dom/client";

import "../assets/palette.css";
import "./index.css";
// Inizializza l'istanza i18next prima del render: useTranslation() nei
// componenti assume che esista già (vedi src/i18n/index.ts).
import "./i18n";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
