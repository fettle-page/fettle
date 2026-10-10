import { useId, useRef, type CSSProperties } from "react";
import type { DetailGroup, ModelLine, ReceiptModel } from "@fettle/brain";
import { usePrintUnfold } from "./usePrintUnfold.ts";
import { Wordmark } from "./Wordmark.tsx";

type Props = { model: ReceiptModel; printing?: boolean; className?: string };

/** The Fettle receipt: a thermal till roll. Same markup in the app, on the share page and in print. */
export function Receipt({ model, printing = false, className }: Props) {
  const uid = useId();
  const root = useRef<HTMLElement>(null);
  usePrintUnfold(root);
  let index = 0;
  const stagger = () => (printing ? ({ "--i": index++ } as CSSProperties) : undefined);

  return (
    <article
      ref={root}
      className={["fettle-receipt", className].filter(Boolean).join(" ")}
      lang={model.locale}
      aria-labelledby={`${uid}-summary`}
      data-tone={model.tone}
      data-printing={printing ? "" : undefined}
    >
      {/* The shadow sits on the article, the torn-edge mask on the paper: a mask would clip the shadow. */}
      <div className="paper">
        <header className="r-head">
          <div className="masthead">
            <Wordmark className="wordmark" />
            <span className="when">{model.meta.when}</span>
          </div>
          {model.machine.length > 0 && (
            <dl className="kv machine">
              {model.machine.map((r) => (
                <div key={r.label}>
                  <dt>
                    <span>{r.label}</span>
                  </dt>
                  <dd>{r.value}</dd>
                </div>
              ))}
            </dl>
          )}
        </header>

        <hr className="rule" />

        <div className="r-summary">
          <h2 id={`${uid}-summary`}>{model.summary}</h2>
          {model.actions.length > 0 && (
            <ol className="steps">
              {model.actions.map((a, i) => (
                <li key={i}>
                  <span className="step-label">{model.labels.nextStep} {i + 1}</span>
                  <span>{a}</span>
                </li>
              ))}
            </ol>
          )}
        </div>

        {model.sections.map((s) => (
          <section key={s.id} className="r-section" data-section={s.id} aria-labelledby={`${uid}-${s.id}`}>
            <h3 id={`${uid}-${s.id}`}>{s.title}</h3>
            <ul>
              {s.lines.map((l) => (
                <Line key={l.id} line={l} style={stagger()} />
              ))}
            </ul>
            {s.details.length > 0 && <Details label={model.labels.details} section={s.title} groups={s.details} />}
          </section>
        ))}

        <hr className="rule" />

        <footer className="r-foot">
          <p>{model.meta.scan}</p>
          <p>{model.meta.footer}</p>
          <div className="barcode" aria-hidden="true" />
          <div className="stamp" aria-hidden="true">
            {model.labels.stamp}
            <small>{model.meta.when}</small>
          </div>
        </footer>
      </div>
    </article>
  );
}

function Line({ line, style }: { line: ModelLine; style: CSSProperties | undefined }) {
  return (
    <li className="line" data-level={line.level} style={style}>
      <span className="sr-only">{line.levelText}: </span>
      <span className="head">{line.headline}</span>
      <span className={`tag tag-${line.level}`} aria-hidden="true">
        {line.tag}
      </span>
      {line.detail && <span className="sub">{line.detail}</span>}
      {line.action && (
        <span className="act">
          <span aria-hidden="true">&gt; </span>
          {line.action}
        </span>
      )}
    </li>
  );
}

function Details({ label, section, groups }: { label: string; section: string; groups: DetailGroup[] }) {
  return (
    <details className="fold">
      {/* Six folds all named "details" would be indistinguishable to a screen reader. */}
      <summary>
        {label}
        <span className="sr-only">: {section}</span>
      </summary>
      {groups.map((g, i) => (
        <div key={i} className="group">
          {g.title && <p className="group-title">{g.title}</p>}
          <dl className="kv">
            {g.rows.map((r, j) => (
              // Startup apps can share a name, so rows are keyed by position.
              <div key={j}>
                <dt>
                    <span>{r.label}</span>
                  </dt>
                <dd>{r.value}</dd>
              </div>
            ))}
          </dl>
        </div>
      ))}
    </details>
  );
}
