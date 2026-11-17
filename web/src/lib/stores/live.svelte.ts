import { SvelteMap } from 'svelte/reactivity';

export type LiveState = 'connecting' | 'live' | 'offline';

export interface LiveSocket {
  url: string;
  onopen: (() => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  onclose: (() => void) | null;
  onerror: (() => void) | null;
  close(): void;
}

export interface LiveUpsert {
  type: string;
  id: string;
  event_kind?: string;
  payload?: Record<string, unknown>;
}

export interface LiveDelete {
  kind: string;
  id: string;
}

export type LiveMessage =
  | { type: 'hello'; seq: number }
  | { type: 'patch'; seq: number; upserts: LiveUpsert[]; deletes: LiveDelete[] }
  | { type: 'snapshot_required'; seq: number };

export interface LiveAgent {
  id: string;
  status: string;
  [key: string]: unknown;
}

export interface LiveAlert {
  id: string;
  state?: string;
  level?: string;
  [key: string]: unknown;
}

const MAX_BACKOFF_MS = 8_000;
const JITTER_MS = 250;

function text(data: unknown): string {
  return typeof data === 'string' ? data : '';
}

function sequence(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

/**
 * Gap-free live snapshot. The only polling here is the reconnect timer after a
 * failed connection; a healthy WebSocket never triggers a periodic refetch.
 */
export function createLiveStore(
  socketOrFactory: LiveSocket | (() => LiveSocket),
  refetch?: () => void | Promise<unknown>
) {
  let state = $state<LiveState>('connecting');
  let seq = $state(0);
  let ticket = $state('');
  let attempts = 0;
  let backoff = MAX_BACKOFF_MS + JITTER_MS;
  let current: LiveSocket | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let manuallyClosed = false;
  let online = true;

  const snapshot = $state({
    agents: new SvelteMap<string, LiveAgent>(),
    alerts: new SvelteMap<string, LiveAlert>(),
    tasks: new SvelteMap<string, LiveUpsert>(),
    connectors: new SvelteMap<string, LiveUpsert>(),
    approvals: new SvelteMap<string, LiveUpsert>(),
    usageStale: false
  });

  const socketFactory =
    typeof socketOrFactory === 'function' ? socketOrFactory : () => socketOrFactory;
  if (typeof socketOrFactory !== 'function') current = socketOrFactory;
  const bucketFor: Record<string, 'agents' | 'alerts' | 'tasks' | 'connectors' | 'approvals'> = {
    agent: 'agents',
    alert: 'alerts',
    task: 'tasks',
    connector: 'connectors',
    approval: 'approvals'
  };

  function clearTimer() {
    if (timer === null) return;
    clearTimeout(timer);
    timer = null;
  }

  function nextBackoff() {
    const base = Math.min(MAX_BACKOFF_MS, MAX_BACKOFF_MS / 32 * 2 ** Math.max(0, attempts - 1));
    backoff = base + Math.floor(Math.random() * JITTER_MS);
  }

  function nextUrl(): string {
    const params = new URLSearchParams();
    if (ticket) params.set('ticket', ticket);
    params.set('since', String(seq));
    return `/api/v1/ws?${params.toString()}`;
  }

  function disconnect() {
    clearTimer();
    manuallyClosed = true;
    if (current) {
      const socket = current;
      current = null;
      socket.onopen = null;
      socket.onmessage = null;
      socket.onclose = null;
      socket.onerror = null;
      socket.close();
    }
    attempts += 1;
    nextBackoff();
    state = 'offline';
  }

  function scheduleReconnect() {
    if (!online || current) return;
    clearTimer();
    timer = setTimeout(() => {
      timer = null;
      void open();
    }, backoff);
  }

  function handleMessage(event: { data: unknown }) {
    let message: LiveMessage;
    try {
      message = JSON.parse(text(event.data)) as LiveMessage;
    } catch {
      invalidate();
      return;
    }
    apply(message);
  }

  function apply(message: LiveMessage) {
    if (!message || typeof message !== 'object') {
      invalidate();
      return;
    }
    if (message.type === 'hello') {
      seq = sequence(message.seq);
      return;
    }
    if (message.type === 'snapshot_required') {
      seq = Math.max(seq, sequence(message.seq));
      invalidate();
      return;
    }
    if (message.type !== 'patch') {
      invalidate();
      return;
    }
    const patchSeq = sequence(message.seq);
    if (patchSeq <= seq) return;
    if (seq > 0 && patchSeq !== seq + 1) {
      invalidate();
      return;
    }
    for (const upsert of message.upserts ?? []) {
      if (upsert.type === 'usage' || upsert.type === 'log') {
        snapshot.usageStale = true;
        continue;
      }
      const bucket = bucketFor[upsert.type];
      if (!bucket || !upsert.id) {
        invalidate();
        return;
      }
      if (bucket === 'agents') {
        snapshot.agents.set(upsert.id, {
          id: upsert.id,
          status: String(upsert.payload?.status ?? 'bekerja'),
          ...upsert.payload
        } as LiveAgent);
      } else if (bucket === 'alerts') {
        snapshot.alerts.set(upsert.id, {
          id: upsert.id,
          ...upsert.payload
        } as LiveAlert);
      } else {
        snapshot[bucket].set(upsert.id, upsert);
      }
    }
    for (const deletion of message.deletes ?? []) {
      const bucket = bucketFor[deletion.kind];
      if (bucket) snapshot[bucket].delete(deletion.id);
    }
    seq = patchSeq;
  }

  function invalidate() {
    const wasConnected = current !== null || manuallyClosed;
    if (current) disconnect();
    else {
      attempts += 1;
      nextBackoff();
      state = 'offline';
    }
    let result: unknown;
    try {
      result = refetch?.();
    } catch {
      result = undefined;
    }
    void Promise.resolve(result)
      .then((value) => {
        if (value && typeof value === 'object' && 'seq' in value) {
          seq = Math.max(seq, sequence(value.seq));
        }
        snapshot.usageStale = false;
        if (wasConnected) {
          state = 'connecting';
          scheduleReconnect();
        }
      })
      .catch(() => {
        if (wasConnected) scheduleReconnect();
      });
  }

  async function open(): Promise<LiveSocket | null> {
    if (!online) {
      state = 'offline';
      return null;
    }
    clearTimer();
    state = 'connecting';
    manuallyClosed = false;
    try {
      const response = await fetch('/api/v1/auth/ws-ticket', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { Accept: 'application/json' }
      });
      if (!response.ok) throw new Error(`ticket ${response.status}`);
      const body = (await response.json()) as { ticket?: string };
      ticket = body.ticket ?? '';
    } catch {
      attempts += 1;
      nextBackoff();
      state = 'offline';
      scheduleReconnect();
      return null;
    }

    const socket = socketFactory();
    current = socket;
    socket.url = nextUrl();
    socket.onopen = () => {
      attempts = 0;
      backoff = 250 + Math.floor(Math.random() * JITTER_MS);
      state = 'live';
    };
    socket.onmessage = handleMessage;
    socket.onclose = () => {
      if (!current || manuallyClosed) return;
      current = null;
      attempts += 1;
      nextBackoff();
      state = 'offline';
      scheduleReconnect();
    };
    socket.onerror = () => {
      if (!current || manuallyClosed) return;
      current = null;
      socket.close();
      attempts += 1;
      nextBackoff();
      state = 'offline';
      scheduleReconnect();
    };
    return socket;
  }

  function setOnline(value: boolean) {
    online = value;
    if (!value) {
      disconnect();
      return;
    }
    attempts = 0;
    backoff = 250 + Math.floor(Math.random() * JITTER_MS);
    state = 'connecting';
    void open();
  }

  return {
    snapshot,
    get state() {
      return state;
    },
    get seq() {
      return seq;
    },
    agent(id: string): LiveAgent | undefined {
      return snapshot.agents.get(id);
    },
    alert(id: string): LiveAlert | undefined {
      return snapshot.alerts.get(id);
    },
    apply,
    open,
    disconnect,
    nextUrl,
    backoffMs: () => backoff,
    setOnline,
    hydrate(value: { seq?: number; agents?: LiveAgent[]; alerts?: LiveAlert[] }) {
      seq = sequence(value.seq);
      for (const item of value.agents ?? []) snapshot.agents.set(item.id, item);
      for (const item of value.alerts ?? []) snapshot.alerts.set(item.id, item);
    }
  };
}
