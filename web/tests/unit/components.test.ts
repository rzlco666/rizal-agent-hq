import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { expect, test } from 'vitest';
import AttentionRail from '$lib/components/domain/AttentionRail.svelte';
import AgentMonogram from '$lib/components/domain/AgentMonogram.svelte';
import Button from '$lib/components/ui/Button.svelte';
import EmptyState from '$lib/components/ui/EmptyState.svelte';
import Keycap from '$lib/components/ui/Keycap.svelte';
import StatusDot from '$lib/components/ui/StatusDot.svelte';
import Table from '$lib/components/ui/Table.svelte';

const label = createRawSnippet(() => ({ render: () => '<span>Simpan</span>' }));

const statuses = [
  'bekerja',
  'berpikir',
  'menunggu-rizal',
  'santai',
  'offline',
  'error'
] as const;

const statusTokens: Record<string, string> = {
  bekerja: 'var(--success)',
  berpikir: 'var(--info)',
  'menunggu-rizal': 'var(--attention)',
  santai: 'var(--ink-10)',
  offline: 'var(--ink-10)',
  error: 'var(--critical)'
};

function tenAlerts() {
  return Array.from({ length: 10 }, (_, index) => ({
    id: `a-${index}`,
    kind: 'alert' as const,
    level: index === 0 || index === 3 ? ('critical' as const) : ('warning' as const),
    title: index === 0 ? 'critical-1' : `alert-${index}`,
    meta: 'subjek · 2m'
  }));
}

test('button renders exactly one primary per surface contract', () => {
  render(Button, { props: { variant: 'primary', children: label } });
  const filled = screen
    .getAllByRole('button')
    .filter((button) => button.getAttribute('data-variant') === 'primary');
  expect(filled).toHaveLength(1);
});

test('status dot maps six statuses to roles', () => {
  for (const status of statuses) {
    const { unmount } = render(StatusDot, { props: { status } });
    expect(screen.getByText(status)).toHaveClass('sr-only');
    expect(document.querySelector('.dot')).toHaveStyle({ background: statusTokens[status] });
    unmount();
  }
});

test('attention rail caps items and groups critical before warning', () => {
  render(AttentionRail, { props: { alerts: tenAlerts(), onopen: () => {} } });
  expect(screen.getAllByTestId('attention-row')).toHaveLength(3);
  expect(screen.getByText('+7')).toBeVisible();
  expect(screen.getAllByTestId('attention-row')[0]).toHaveTextContent('critical-1');
});

test('agent monogram uses initials without per-agent hue', () => {
  render(AgentMonogram, { props: { name: 'Rizal Agent' } });
  const image = screen.getByRole('img');
  expect(image).toHaveAttribute('aria-label', 'Rizal Agent');
  expect(image).toHaveTextContent('RA');
  expect(document.querySelector('rect')).toHaveAttribute('fill', 'var(--well)');
});

test('empty state always renders two lines', () => {
  render(EmptyState, { props: { title: 'Judul kosong', body: 'Isi penjelasan' } });
  expect(screen.getByText('Judul kosong')).toBeVisible();
  expect(screen.getByText('Isi penjelasan')).toBeVisible();
});

test('keycap renders kbd semantics', () => {
  const { container } = render(Keycap, { props: { keys: ['Meta', 'K'] } });
  expect(container.querySelectorAll('kbd')).toHaveLength(2);
});

test('table row is reachable without clicking the row', () => {
  render(Table, {
    props: {
      columns: [
        { key: 'name', label: 'Nama' },
        { key: 'n', label: 'Jumlah', numeric: true }
      ],
      rows: [{ id: '1', name: 'Alpha', n: 1234 }],
      caption: 'Daftar agen',
      rowHref: (row) => `/agents/${row.id}`
    }
  });
  expect(screen.getByRole('link', { name: 'Alpha' })).toHaveAttribute('href', '/agents/1');
});
