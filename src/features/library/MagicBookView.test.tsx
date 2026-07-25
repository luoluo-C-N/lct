import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MagicBookView } from './MagicBookView';

it('replaces the date flow after selecting a month', async () => {
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} />);
  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '5 月' }));
  expect(await screen.findByRole('button', { name: /05 \/ 18/ })).toBeTruthy();
});
