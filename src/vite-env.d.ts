/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_IPC_MOCK?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
