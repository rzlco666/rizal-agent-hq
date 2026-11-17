import { describe, expect, test, vi } from 'vitest';
import { createLiveStore, type LiveSocket } from './live.svelte';

function fakeSocket(): LiveSocket {
  return {
    url: '',
    onopen: null,
    onmessage: null,
    onclose: null,
    onerror: null,
    closed: false,
    close() {
      this.closed = true;
      this.onclose?.();
    }
  };
}

function agent(id: string, status: string) {
  return {
    type: 'agent',
    id,
    event_kind: 'agent_status_changed',
    payload: { status }
  };
}

describe('live store', () => {
  test('applies patch and tracks seq', () => {
    const socket = fakeSocket();
    const store = createLiveStore(socket);
    store.apply({ type: 'hello', seq: 10 });
    store.apply({
      type: 'patch',
      seq: 11,
      upserts: [agent('agent:omp', 'bekerja')],
      deletes: []
    });
    expect(store.seq).toBe(11);
    expect(store.agent('agent:omp')?.status).toBe('bekerja');
    expect(store.state).toBe('connecting');
  });

  test('ignores duplicate patches and removes deleted entities', () => {
    const store = createLiveStore(fakeSocket());
    store.apply({ type: 'hello', seq: 1 });
    store.apply({
      type: 'patch',
      seq: 2,
      upserts: [agent('agent:omp', 'error')],
      deletes: []
    });
    store.apply({
      type: 'patch',
      seq: 2,
      upserts: [agent('agent:omp', 'santai')],
      deletes: []
    });
    expect(store.agent('agent:omp')?.status).toBe('error');
    store.apply({
      type: 'patch',
      seq: 3,
      upserts: [],
      deletes: [{ kind: 'agent', id: 'agent:omp' }]
    });
    expect(store.agent('agent:omp')).toBeUndefined();
    expect(store.seq).toBe(3);
  });

  test('snapshot_required triggers refetch', () => {
    const refetch = vi.fn();
    const store = createLiveStore(fakeSocket(), refetch);
    store.apply({ type: 'snapshot_required', seq: 99 });
    expect(refetch).toHaveBeenCalledOnce();
  });

  test('seq gap triggers refetch', () => {
    const refetch = vi.fn();
    const store = createLiveStore(fakeSocket(), refetch);
    store.apply({ type: 'hello', seq: 4 });
    store.apply({ type: 'patch', seq: 9, upserts: [], deletes: [] });
    expect(refetch).toHaveBeenCalledOnce();
    expect(store.seq).toBe(4);
  });

  test('reconnect uses last seq and backs off', () => {
    const socket = fakeSocket();
    const store = createLiveStore(socket);
    store.apply({ type: 'hello', seq: 42 });
    store.disconnect();
    expect(socket.closed).toBe(true);
    expect(store.state).toBe('offline');
    expect(store.nextUrl()).toContain('since=42');
    expect(store.backoffMs()).toBeGreaterThanOrEqual(250);
    expect(store.backoffMs()).toBeLessThanOrEqual(8250);
  });

  test('offline event moves state to offline and online back to connecting', () => {
    const store = createLiveStore(fakeSocket());
    store.setOnline(false);
    expect(store.state).toBe('offline');
    store.setOnline(true);
    expect(store.state).toBe('connecting');
  });
});
