import type { DiscoveredClient } from '@/lib/tauri';

/**
 * Clients that are both actually detected on this machine AND explicitly
 * managed by the user (per `config.managed_client_ids`). Use this instead of
 * a bare `.filter(c => c.detected)` anywhere a client list is offered as a
 * deploy/adopt target, so an unmanaged-but-detected client never appears.
 */
export function managedAndDetected(
  clients: DiscoveredClient[],
  managedIds: string[],
): DiscoveredClient[] {
  return clients.filter((c) => c.detected && managedIds.includes(c.id));
}
