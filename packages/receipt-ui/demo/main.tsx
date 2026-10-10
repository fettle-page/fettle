import { toLocale } from "@fettle/brain";
import { createBrain } from "@fettle/brain/core";
import wasmUrl from "@fettle/brain/fettle.wasm?url";
import { StrictMode, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { ReceiptView, Wordmark } from "../src/index.ts";
// Fonts are self-hosted (OFL), never loaded from a third-party CDN.
import "@fontsource-variable/bricolage-grotesque/opsz.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "@fontsource/ibm-plex-mono/600.css";
import "@fontsource/ibm-plex-mono/700.css";
import "./demo.css";
import { FIXTURES } from "./fixtures.ts";

// The Rust receipt, loaded in this test page's browser (the website never does this: its pages get
// receipts finished by the server).
const brain = createBrain(await WebAssembly.compileStreaming(fetch(wasmUrl)));

function Demo() {
  const params = new URLSearchParams(location.search);
  const [id, setId] = useState(params.get("fixture") ?? FIXTURES[0]!.id);
  const [locale, setLocale] = useState(params.get("lang") ?? "en");
  const [run, setRun] = useState(0);

  const fixture = FIXTURES.find((f) => f.id === id) ?? FIXTURES[0]!;
  const model = useMemo(() => {
    const { report } = fixture;
    // In this machine's own zone at the scan time, as the app shows it.
    return brain.receiptFor(report, toLocale(locale), { offsetMinutes: -new Date(report.collected_at).getTimezoneOffset() }).model;
  }, [fixture, locale]);

  return (
    <div className="mat">
      <div className="ruler" aria-hidden="true" />
      <header className="bar">
        <Wordmark className="brand" />
        <span className="tape quiet">receipt · test page</span>
      </header>

      <div className="controls" role="group" aria-label="Test page controls">
        <label className="tape">
          <span className="sr-only">Report</span>
          <select value={id} onChange={(e) => setId(e.target.value)}>
            {FIXTURES.map((f) => (
              <option key={f.id} value={f.id}>{f.label}</option>
            ))}
          </select>
        </label>
        <button className="tape" type="button" aria-pressed={locale === "it"} onClick={() => setLocale(locale === "en" ? "it" : "en")}>
          {locale === "en" ? "EN → IT" : "IT → EN"}
        </button>
        <button className="tape" type="button" onClick={() => setRun((n) => n + 1)}>Print again</button>
        <button className="tape light" type="button" onClick={() => window.print()}>Print / PDF</button>
      </div>

      <main className="stage">
        <ReceiptView key={`${id}-${locale}-${run}`} model={model} printing />
      </main>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Demo />
  </StrictMode>,
);
