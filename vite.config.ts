import path from "path"
import { readFileSync } from "fs"
import { defineConfig, type UserConfig } from "vite"
import react from "@vitejs/plugin-react"
import tailwindcss from "@tailwindcss/vite"

const host = process.env.TAURI_DEV_HOST

// Read version from package.json at config-load time so the Settings
// UI can show the running app version without duplicating the string.
const pkgJson = JSON.parse(readFileSync(path.resolve(__dirname, "package.json"), "utf-8"))

// `npm run dev:web`: the browser build against a local llm-wiki-server,
// proxied so the session cookie stays same-origin.
const webServerTarget = process.env.LLM_WIKI_SERVER ?? "http://127.0.0.1:19830"
const webServerProxy = Object.fromEntries(
  ["/rpc", "/events", "/files", "/proxy", "/login", "/logout"].map((path) => [path, { target: webServerTarget }]),
)

// `--mode worker`: the Node ingest worker llm-wiki-server supervises,
// bundled with its dependencies into dist-worker/ingest-worker.mjs.
const workerBuild: UserConfig["build"] = {
  ssr: "src/worker/main.ts",
  outDir: "dist-worker",
  target: "node22",
  rolldownOptions: { output: { entryFileNames: "ingest-worker.mjs" } },
}

// https://vitejs.dev/config/
export default defineConfig(async ({ mode }) => ({
  plugins: mode === "worker" ? [] : [react(), tailwindcss()],

  resolve: {
    alias: [
      // In the browser build the ingest queue runs in the server's worker;
      // the tab gets a remote proxy with the same API.
      ...(mode === "web"
        ? [{ find: /^@\/lib\/ingest-queue$/, replacement: path.resolve(__dirname, "./src/lib/ingest-queue-remote.ts") }]
        : []),
      { find: "@", replacement: path.resolve(__dirname, "./src") },
    ],
  },

  build: mode === "worker" ? workerBuild : undefined,
  ssr: mode === "worker" ? { noExternal: true as const } : undefined,

  define: {
    __APP_VERSION__: JSON.stringify(pkgJson.version),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: mode === "web" ? 1430 : 1420,
    strictPort: true,
    proxy: mode === "web" ? webServerProxy : undefined,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  test: {
    environment: "node",
    // Loads .env.test.local into process.env for real-LLM tests.
    // The loader itself is a no-op if the file is absent, so this is
    // safe to keep on for every test run.
    setupFiles: ["./src/test-helpers/load-test-env.ts"],
  },
}))
