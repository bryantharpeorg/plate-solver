import type { McpStatus } from "../api";

export function McpSection({ status }: { status: unknown }) {
  if (typeof status !== "object" || status === null) return null;
  const info = status as Partial<McpStatus>;
  if (typeof info.enabled !== "boolean" || !Array.isArray(info.urls)) return null;
  const hosts = Array.isArray(info.allowed_hosts) ? info.allowed_hosts : [];
  return (
    <section aria-labelledby="setup-mcp-title" className="space-y-3">
      <h2 id="setup-mcp-title" className="section-title">
        MCP
      </h2>
      <div className="space-y-3 border-y border-rule py-4">
        {!info.enabled ? (
          <p>MCP is off.</p>
        ) : (
          <>
            <p>Paste an endpoint URL into your MCP client:</p>
            <ul aria-label="MCP URLs" className="space-y-1 break-all">
              {info.urls.map((url, i) => (
                <li key={i}>{url}</li>
              ))}
            </ul>
            {info.local_only && (
              <p className="quiet">
                Reachable only from this machine. MCP clients on other devices can't reach it until the console listens
                on the LAN.
              </p>
            )}
            <p className="quiet">
              Under WSL's NAT, the phone needs the README's “MCP under WSL2: portproxy or mirrored mode” setup; the WSL
              address is not the Surface's LAN address.
            </p>
          </>
        )}
        <p className="text-sm text-ink-2">
          Allowed host names:{" "}
          {hosts.length
            ? hosts.map((host, i) => (
                <span key={i}>
                  {i > 0 && ", "}
                  {host}
                </span>
              ))
            : "None configured."}
        </p>
      </div>
    </section>
  );
}
