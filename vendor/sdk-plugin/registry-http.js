export class HttpPluginRegistry {
    baseUrl;
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }
    get url() { return this.baseUrl; }
    async bundleUrl(id, version) {
        return `${this.baseUrl}/v1/plugins/${id}/${version}/web.js`;
    }
    async checkLatest(id, currentVersion) {
        const res = await fetch(`${this.baseUrl}/v1/plugins/${id}/latest`);
        if (!res.ok) {
            throw new Error(`checkLatest failed: ${res.status} ${res.statusText}`);
        }
        const { version } = (await res.json());
        return version !== currentVersion ? { version } : null;
    }
    /** Check reachability, plugin count, and optional server version. Never throws. */
    async checkHealth() {
        const start = Date.now();
        try {
            const res = await fetch(`${this.baseUrl}/v1/index`);
            const latencyMs = Date.now() - start;
            if (!res.ok)
                return { online: false, pluginCount: 0, latencyMs };
            const data = (await res.json());
            return { online: true, pluginCount: data.plugins.length, version: data.version, latencyMs };
        }
        catch {
            return { online: false, pluginCount: 0, latencyMs: Date.now() - start };
        }
    }
    /** Fetch all plugins from the registry index. Returns empty array on any failure. */
    async listPlugins() {
        try {
            const res = await fetch(`${this.baseUrl}/v1/index`);
            if (!res.ok)
                return [];
            const { plugins } = (await res.json());
            return plugins.map(p => ({
                id: p.id,
                registry: this,
                installedVersion: p.latestVersion,
                latestVersion: p.latestVersion,
                pluginTypes: p.pluginTypes,
            }));
        }
        catch {
            return [];
        }
    }
}
