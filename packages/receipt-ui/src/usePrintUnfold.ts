import { useEffect, type RefObject } from "react";

/** Opens every closed details fold inside `ref` for printing, and closes only those again afterwards. */
export function usePrintUnfold(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    let opened: HTMLDetailsElement[] = [];
    const before = () => {
      opened = [...(ref.current?.querySelectorAll("details") ?? [])].filter((d) => !d.open);
      for (const d of opened) d.open = true;
    };
    const after = () => {
      for (const d of opened) d.open = false;
      opened = [];
    };
    window.addEventListener("beforeprint", before);
    window.addEventListener("afterprint", after);
    return () => {
      window.removeEventListener("beforeprint", before);
      window.removeEventListener("afterprint", after);
    };
  }, [ref]);
}
