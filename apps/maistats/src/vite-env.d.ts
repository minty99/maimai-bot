/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly SONG_INFO_SERVER_URL?: string;
  readonly RECORD_COLLECTOR_SERVER_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
