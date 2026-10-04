/// <reference types="vite/client" />

/** App version injected by Vite's `define` from package.json. */
declare const __APP_VERSION__: string

interface ImportMetaEnv {
  /**
   * "web": browser frontend for the self-hosted server; "worker": the Node
   * ingest worker it supervises; unset = desktop.
   */
  readonly VITE_LLM_WIKI_PLATFORM?: "web" | "worker"
  /** Server origin for the web build; empty = same origin as the page. */
  readonly VITE_LLM_WIKI_SERVER_URL?: string
}
