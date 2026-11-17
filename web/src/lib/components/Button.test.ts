import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { expect, test } from 'vitest';
import Button from './ui/Button.svelte';

const label = createRawSnippet(() => ({ render: () => '<span>Simpan</span>' }));

test('primary is the only filled action', () => {
  render(Button, { props: { variant: 'primary', children: label } });
  expect(screen.getByRole('button')).toHaveAttribute('data-variant', 'primary');
});

test('disabled button is not clickable', async () => {
  render(Button, { props: { disabled: true, children: label } });
  expect(screen.getByRole('button')).toBeDisabled();
});
