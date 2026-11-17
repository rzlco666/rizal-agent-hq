import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';
import Tabs from './Tabs.svelte';

test('tabs support arrow keys and aria', async () => {
  const user = userEvent.setup();
  render(Tabs, {
    props: {
      tabs: [
        { id: 'a', label: 'A' },
        { id: 'b', label: 'B' }
      ],
      value: 'a'
    }
  });
  const a = screen.getByRole('tab', { name: 'A' });
  a.focus();
  await user.keyboard('{ArrowRight}');
  expect(screen.getByRole('tab', { name: 'B' })).toHaveAttribute('aria-selected', 'true');
});
