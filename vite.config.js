import { cloudflare } from '@cloudflare/vite-plugin';
import { defineConfig, loadEnv } from 'vite';
export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, process.cwd(), '');
    return {
        plugins: [cloudflare()],
        define: {
            'import.meta.env.SONG_INFO_SERVER_URL': JSON.stringify(env.SONG_INFO_SERVER_URL ?? ''),
            'import.meta.env.RECORD_COLLECTOR_SERVER_URL': JSON.stringify(env.RECORD_COLLECTOR_SERVER_URL ?? ''),
        },
        server: {
            host: true,
            port: 5174,
        },
    };
});
