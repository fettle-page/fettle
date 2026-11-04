declare module "*.wasm" {
  const module: WebAssembly.Module;
  export default module;
}

declare module "*.wasm?url" {
  const url: string;
  export default url;
}
