/**
 * Signal for an expired session. The API client reports every request that ran into the login (redirect,
 * login page or status 401); the application frame listens once and offers to sign in again. Kept apart
 * from the client so pages can show their own error text while the frame handles the sign-in.
 */

/** Listener called when a request found the session expired. */
type Listener = () => void;

const listeners = new Set<Listener>();

/** Tells all listeners that a request found the session expired. */
export function notifySessionExpired(): void {
  for (const listener of listeners) listener();
}

/**
 * Registers a listener for an expired session.
 * @returns Function that removes the listener again (for effect cleanup).
 */
export function onSessionExpired(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
