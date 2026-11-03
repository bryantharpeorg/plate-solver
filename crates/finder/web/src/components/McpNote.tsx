import type { SessionMotion } from "../api";

export function McpNote({ motion }: { motion?: SessionMotion }) {
  if (motion?.by !== "mcp") return null;
  return <p className="min-w-0 break-words text-sm text-ink-2">Moved by an MCP client · {motion.tool}</p>;
}
