import type { RegistryPlugin } from './types.js';

interface WebPluginEntry {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  downloads: number;
  author: string;
  tags: string[];
}

interface WebRegistryIndex {
  version: number;
  updatedAt: number;
  plugins: WebPluginEntry[];
}

interface WebSearchResults {
  plugins: WebPluginEntry[];
}

const toRegistryPlugin = (p: WebPluginEntry): RegistryPlugin => ({
  id: p.id,
  name: p.name,
  description: p.description,
  latestVersion: p.latestVersion,
  downloads: p.downloads,
  author: p.author,
  tags: p.tags ?? [],
  pluginTypes: ['web'],
});

/** Search plugins from a registry. Returns matching subset. */
export const searchPlugins = async (
  registryUrl: string,
  query: string,
): Promise<RegistryPlugin[]> => {
  const url = query.trim()
    ? `${registryUrl}/v1/search?q=${encodeURIComponent(query)}`
    : `${registryUrl}/v1/index.json`;

  const res = await fetch(url);
  if (!res.ok) return [];

  const data = (await res.json()) as WebSearchResults;
  return (data.plugins ?? []).map(toRegistryPlugin);
};

/** Fetch full list from all registries. */
export const fetchAllPlugins = async (
  registryUrls: string[],
): Promise<RegistryPlugin[]> => {
  const results = await Promise.allSettled(
    registryUrls.map(async (url) => {
      const res = await fetch(`${url}/v1/index.json`);
      if (!res.ok) return [];
      const data = (await res.json()) as WebRegistryIndex;
      return (data.plugins ?? []).map(toRegistryPlugin);
    }),
  );

  const seen = new Set<string>();
  return results.flatMap((r) =>
    r.status === 'fulfilled'
      ? r.value.filter((p) => {
          if (seen.has(p.id)) return false;
          seen.add(p.id);
          return true;
        })
      : [],
  );
};

/** Execute a command on a cocoon via silk session and collect output. */
export const executeOnCocoon = (
  session: { execute: (cmd: string) => { onOutput: (fn: (stream: string, data: string) => void) => () => void; onCompleted: (fn: (exitCode: number) => void) => () => void; onError: (fn: (code: string, msg: string) => void) => () => void } },
  command: string,
): Promise<{ exitCode: number; output: string }> =>
  new Promise((resolve, reject) => {
    const lines: string[] = [];
    const cmd = session.execute(command);

    cmd.onOutput((_stream, data) => {
      lines.push(data);
    });

    cmd.onCompleted((exitCode) => {
      resolve({ exitCode, output: lines.join('') });
    });

    cmd.onError((_code, msg) => {
      reject(new Error(msg));
    });
  });

/** Parse `adi plugin installed` output into a map of pluginId → version. */
export const parseInstalledPlugins = (output: string): Map<string, string> => {
  const installed = new Map<string, string>();
  for (const line of output.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('─') || trimmed.startsWith('Plugin') || trimmed.startsWith('No ')) continue;
    // Expected format: "plugin.id  1.0.0" or "plugin.id 1.0.0 /path"
    const parts = trimmed.split(/\s+/);
    if (parts.length >= 2) {
      installed.set(parts[0], parts[1]);
    }
  }
  return installed;
};
