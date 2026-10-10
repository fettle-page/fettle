import { Receipt } from "./Receipt.tsx";
import type { ReceiptModel } from "@fettle/brain";
import { Workbench } from "./Workbench.tsx";

/**
 * The receipt with its workbench. Sizes follow the space this view gets, not the window:
 * a narrow roll in small spaces, a wide roll from 900px, the workbench beside it from 1100px.
 * The view needs a parent with a definite width (it is a size container), and no ancestor
 * with overflow hidden or auto between it and the page scroll, or the workbench stops sticking.
 */
export function ReceiptView({ model, printing = false }: { model: ReceiptModel; printing?: boolean }) {
  return (
    <div className="fettle-view">
      <div className="fettle-view-layout">
        <Receipt model={model} printing={printing} />
        {/* A new report starts a fresh checklist; switching language keeps the ticks. */}
        <Workbench key={model.meta.collectedAt} model={model} />
      </div>
    </div>
  );
}
