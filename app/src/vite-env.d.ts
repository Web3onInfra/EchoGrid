/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_ORACLE_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
