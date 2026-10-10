import { useId, useRef, useState, type KeyboardEvent } from "react";
import { uiText as ui } from "@fettle/brain";
import type { Level, ModelSection, ReceiptModel } from "@fettle/brain";

const SEVERITY: Level[] = ["info", "fine", "couldnt_check", "keep_an_eye", "act_soon"];

/** The most serious level among a section's lines. */
export function worstLevel(section: ModelSection): Level {
  return section.lines.reduce<Level>((worst, l) => (SEVERITY.indexOf(l.level) > SEVERITY.indexOf(worst) ? l.level : worst), "info");
}

/** Where the numbers open: the first section needing action, then one to watch, else the first. */
function firstToShow(sections: ModelSection[]): ModelSection | undefined {
  return sections.find((s) => worstLevel(s) === "act_soon") ?? sections.find((s) => worstLevel(s) === "keep_an_eye") ?? sections[0];
}

/** Beside the receipt on wide screens: the next steps as a checklist, and each section's numbers. */
export function Workbench({ model }: { model: ReceiptModel }) {
  const uid = useId();
  const withNumbers = model.sections.filter((s) => s.details.length > 0);
  const [active, setActive] = useState(() => firstToShow(withNumbers)?.id);
  const [done, setDone] = useState<ReadonlySet<number>>(new Set());
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);

  if (model.actions.length === 0 && withNumbers.length === 0) return null;
  const current = withNumbers.find((s) => s.id === active) ?? withNumbers[0];
  const locale = model.locale;

  const toggle = (i: number) =>
    setDone((d) => {
      const next = new Set(d);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });

  // Tabs follow the ARIA pattern: arrows move and select, Home/End jump, one tab in the tab order.
  const onKey = (e: KeyboardEvent, index: number) => {
    const last = withNumbers.length - 1;
    const to = { ArrowRight: index === last ? 0 : index + 1, ArrowLeft: index === 0 ? last : index - 1, Home: 0, End: last }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    setActive(withNumbers[to]!.id);
    tabRefs.current[to]?.focus();
  };

  return (
    <aside className="fettle-bench" lang={locale} aria-label={ui("bench.todo", locale)}>
      <section className="bench-block">
        <h2>{ui("bench.todo", locale)}</h2>
        {model.actions.length === 0 ? (
          <p className="bench-none">{ui("bench.none", locale)}</p>
        ) : (
          <ol className="checklist">
            {model.actions.map((a, i) => (
              <li key={i} data-done={done.has(i) ? "" : undefined}>
                <label>
                  <input type="checkbox" checked={done.has(i)} onChange={() => toggle(i)} />
                  <span>{a}</span>
                </label>
              </li>
            ))}
          </ol>
        )}
      </section>

      {current && (
        <section className="bench-block">
          <h2>{ui("bench.numbers", locale)}</h2>
          <div className="bench-tabs" role="tablist" aria-label={ui("bench.sections", locale)}>
            {withNumbers.map((s, i) => {
              const level = worstLevel(s);
              const selected = s.id === current.id;
              return (
                <button
                  key={s.id}
                  ref={(el) => {
                    tabRefs.current[i] = el;
                  }}
                  id={`${uid}-tab-${s.id}`}
                  role="tab"
                  type="button"
                  aria-selected={selected}
                  aria-controls={`${uid}-panel`}
                  tabIndex={selected ? 0 : -1}
                  data-level={level}
                  onClick={() => setActive(s.id)}
                  onKeyDown={(e) => onKey(e, i)}
                >
                  <span className="mark" aria-hidden="true" />
                  <span className="sr-only">{s.lines.find((l) => l.level === level)?.levelText}: </span>
                  {s.title}
                </button>
              );
            })}
          </div>
          <div id={`${uid}-panel`} role="tabpanel" className="bench-panel" aria-labelledby={`${uid}-tab-${current.id}`} tabIndex={0}>
            {current.details.map((g, i) => (
              <div key={i} className="bench-group">
                {g.title && <p className="bench-group-title">{g.title}</p>}
                <dl>
                  {g.rows.map((r, j) => (
                    <div key={j}>
                      <dt>{r.label}</dt>
                      <dd>{r.value}</dd>
                    </div>
                  ))}
                </dl>
              </div>
            ))}
          </div>
        </section>
      )}
    </aside>
  );
}
