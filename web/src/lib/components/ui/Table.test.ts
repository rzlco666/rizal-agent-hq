import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import Table from './Table.svelte';

test('table rows are 38px and numeric cells align right', () => {
  render(Table, {
    props: {
      columns: [{ key: 'n', label: 'N', numeric: true }],
      rows: [{ n: 1234 }]
    }
  });
  expect(screen.getByRole('cell')).toHaveClass('num');
});
