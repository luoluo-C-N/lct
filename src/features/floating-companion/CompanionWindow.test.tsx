import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CompanionWindow } from './CompanionWindow';

it('opens capture actions', async () => {
  render(<CompanionWindow />);
  await userEvent.click(screen.getByRole('button', { name: '悬浮角色' }));
  expect(screen.getByRole('button', { name: '区域截图' })).toBeTruthy();
});
