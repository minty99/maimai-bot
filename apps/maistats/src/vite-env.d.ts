/// <reference types="vite/client" />

declare module 'plotly.js-dist-min' {
  const Plotly: {
    react(
      root: HTMLDivElement,
      data: Array<Record<string, unknown>>,
      layout?: Record<string, unknown>,
      config?: Record<string, unknown>,
    ): Promise<void>;
    purge(root: HTMLDivElement): void;
  };
  export default Plotly;
}

interface ImportMetaEnv {
  readonly SONG_DATABASE_URL?: string;
  readonly RECORD_COLLECTOR_SERVER_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
