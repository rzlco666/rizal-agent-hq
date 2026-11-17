import { beforeEach, expect, test, vi } from 'vitest';
import { ackAlert, snoozeAlert } from '$lib/api/client';
import { t } from '$lib/i18n';
import { attention } from './attention.svelte';
import { toasts } from './toast.svelte';

vi.mock('$lib/api/client', () => ({
  ackAlert: vi.fn(),
  snoozeAlert: vi.fn(),
  resolveAlert: vi.fn()
}));

const ackMock = vi.mocked(ackAlert);
const snoozeMock = vi.mocked(snoozeAlert);

function alert(id: string, level = 'warning') {
  return {
    id,
    fingerprint: id,
    rule_id: 'rule',
    level,
    state: 'open',
    subject: 'agent:omp',
    title: `Alert ${id}`,
    opened_ms: Date.now() - 60_000,
    updated_ms: Date.now(),
    ack_ms: null,
    snooze_until_ms: null,
    resolved_ms: null,
    occurrences: 1
  };
}

beforeEach(() => {
  ackMock.mockReset();
  snoozeMock.mockReset();
  toasts.items = [];
  attention.items = [];
  attention.openCount = 0;
});

test('ack removes alert from rail and count optimistically', async () => {
  attention.setAlerts([alert('alert:a'), alert('alert:b')], 2);
  ackMock.mockResolvedValue({
    id: 'alert:a',
    state: 'acknowledged',
    seq: 3,
    snooze_until_ms: null
  });
  await attention.ack('alert:a');
  expect(attention.items.map((item) => item.id)).toEqual(['alert:b']);
  expect(attention.openCount).toBe(1);
});

test('snooze failure shows a snooze-specific toast', async () => {
  attention.setAlerts([alert('alert:a')], 1);
  snoozeMock.mockRejectedValue(new Error('network'));
  await attention.snooze('alert:a');
  expect(toasts.items.at(-1)?.message).toBe(t('toast.snooze_failed'));
  expect(attention.openCount).toBe(1);
});

test('ack rolls back and reports failure when request rejects', async () => {
  const failure = vi.spyOn(console, 'error').mockImplementation(() => {});
  attention.setAlerts([alert('alert:a'), alert('alert:b')], 2);
  ackMock.mockRejectedValue(new Error('network'));
  await attention.ack('alert:a');
  expect(attention.items.map((item) => item.id)).toEqual(['alert:a', 'alert:b']);
  expect(attention.openCount).toBe(2);
  failure.mockRestore();
});
