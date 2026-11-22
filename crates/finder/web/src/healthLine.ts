import type { Health } from "./api";

export function healthLine(health: Health): string {
  if (health.databases && health.databases.length > 1) {
    const databases = health.databases
      .map((database) => `${database.name}, fields ${database.min_fov}–${database.max_fov}°`)
      .join("; ");
    return `Databases: ${databases} · finder ${health.version}`;
  }
  return `Database: ${health.database.catalog}, ${health.database.stars.toLocaleString("en-US")} stars, fields ${health.database.min_fov}–${health.database.max_fov}° · finder ${health.version}`;
}
