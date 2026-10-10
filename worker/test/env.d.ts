// Types `exports` from "cloudflare:workers" as this Worker's own module (wrangler types leaves it empty).
declare namespace Cloudflare {
  interface GlobalProps {
    mainModule: typeof import("../src/index.ts");
  }
}

declare module "*?raw" {
  const text: string;
  export default text;
}
