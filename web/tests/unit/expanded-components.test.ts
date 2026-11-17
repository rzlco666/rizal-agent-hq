import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { createRawSnippet } from 'svelte';
import { expect, test, vi } from 'vitest';
import KpiFigure from '$lib/components/ui/KpiFigure.svelte';
import Meter from '$lib/components/ui/Meter.svelte';
import Modal from '$lib/components/ui/Modal.svelte';
import PageHeader from '$lib/components/ui/PageHeader.svelte';
import Pagination from '$lib/components/ui/Pagination.svelte';
import Section from '$lib/components/ui/Section.svelte';
import Switch from '$lib/components/ui/Switch.svelte';
import LedgerChart from '$lib/components/domain/LedgerChart.svelte';

const children = createRawSnippet(() => ({ render: () => '<span>Konten</span>' }));

test('page header renders one h1 with meta and actions', () => {
  render(PageHeader, {
    props: {
      title: 'Hari ini',
      description: 'Ringkasan kerja',
      meta: 'Senin, 17 November',
      actions: children
    }
  });
  expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
  expect(screen.getByText('Ringkasan kerja')).toBeVisible();
  expect(screen.getByText('Senin, 17 November')).toBeVisible();
  expect(screen.getByText('Konten')).toBeVisible();
});

test('section honours heading level', () => {
  const { unmount } = render(Section, { props: { title: 'Agen', level: 2, children } });
  expect(screen.getByRole('heading', { level: 2, name: 'Agen' })).toBeVisible();
  unmount();
  render(Section, { props: { title: 'Detail', level: 3, children } });
  expect(screen.getByRole('heading', { level: 3, name: 'Detail' })).toBeVisible();
});

test('kpi figure exposes label, value and comparison', () => {
  render(KpiFigure, {
    props: {
      label: 'Biaya',
      value: '12,50',
      unit: 'USD',
      delta: '+12%',
      compare: 'vs kemarin'
    }
  });
  expect(screen.getByText('Biaya')).toBeVisible();
  expect(screen.getByText('12,50')).toBeVisible();
  expect(screen.getByText('+12% vs kemarin')).toBeVisible();
});

test('meter maps thresholds to progressbar semantics and levels', () => {
  const { unmount } = render(Meter, { props: { value: 60, max: 100, label: 'Kuota' } });
  const bar = screen.getByRole('progressbar', { name: 'Kuota' });
  expect(bar).toHaveAttribute('aria-valuenow', '60');
  expect(bar).toHaveAttribute('data-level', 'normal');
  unmount();
  render(Meter, { props: { value: 100, max: 100, label: 'Kuota' } });
  expect(screen.getByRole('progressbar')).toHaveAttribute('data-level', 'critical');
});

test('switch toggles by click and Space and reports state', async () => {
  const user = userEvent.setup();
  const onchange = vi.fn();
  render(Switch, { props: { checked: false, label: 'Otomasi', onchange } });
  const control = screen.getByRole('switch', { name: 'Otomasi' });
  expect(control).toHaveAttribute('aria-checked', 'false');
  await user.click(control);
  expect(onchange).toHaveBeenLastCalledWith(true);
  control.focus();
  await user.keyboard(' ');
  expect(onchange).toHaveBeenLastCalledWith(false);
});

test('pagination reports range and disables boundary buttons', async () => {
  const user = userEvent.setup();
  const onchange = vi.fn();
  render(Pagination, { props: { page: 1, pageSize: 25, total: 120, onchange } });
  expect(screen.getByText('1–25 dari 120')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Sebelumnya' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: 'Berikutnya' }));
  expect(onchange).toHaveBeenCalledWith(2);
});

test('confirm dialog focuses enabled cancel button while busy', async () => {
  render(Modal, {
    props: {
      open: true,
      title: 'Hapus agen',
      size: 'confirm',
      confirmTone: 'destructive',
      busy: true,
      onclose: () => {},
      onconfirm: () => {},
      children
    }
  });
  const cancel = screen.getByRole('button', { name: 'Batal' });
  expect(cancel).toBeEnabled();
  await waitFor(() => expect(cancel).toHaveFocus());
});

const points = [
  { ts_ms: Date.parse('2026-01-01T00:00:00Z'), cash_usd: 1, shadow_usd: 2 },
  { ts_ms: Date.parse('2026-01-01T01:00:00Z'), cash_usd: 3, shadow_usd: 4 }
];

test('ledger chart exposes summary, data table and keyboard navigation', async () => {
  const user = userEvent.setup();
  render(LedgerChart, {
    props: { points, summary: 'Biaya dua titik', height: 240, state: 'ready' }
  });
  expect(screen.getByRole('figure', { name: 'Biaya dua titik' })).toBeVisible();
  const chart = screen.getByRole('img', { name: /grafik biaya/i });
  chart.focus();
  expect(chart).toHaveAttribute('data-active', '0');
  await user.keyboard('{ArrowRight}');
  expect(chart).toHaveAttribute('data-active', '1');
  await user.click(screen.getByRole('button', { name: 'Tabel' }));
  expect(screen.getByRole('table', { name: 'Biaya dua titik' })).toBeVisible();
});

test('ledger chart empty and error states show two lines and retry', () => {
  const retry = vi.fn();
  const { unmount } = render(LedgerChart, {
    props: { points: [], summary: 'Kosong', state: 'empty' }
  });
  expect(screen.getByText('Tidak ada data')).toBeVisible();
  expect(screen.getByText('Grafik muncul setelah data tersedia.')).toBeVisible();
  unmount();
  render(LedgerChart, {
    props: { points: [], summary: 'Gagal', state: 'error', onretry: retry }
  });
  expect(screen.getByText('Grafik gagal dimuat')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Coba lagi' })).toBeVisible();
});
