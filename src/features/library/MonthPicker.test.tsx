import '@testing-library/jest-dom/vitest';
import { useState } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MonthPicker } from './MonthPicker';

function MonthPickerHarness() {
  const [month, setMonth] = useState({ year: 2026, month: 7 });
  return <MonthPicker month={month} onSelect={setMonth} />;
}

it('selects a month from the next year and closes the album shelf', async () => {
  render(<MonthPickerHarness />);

  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '下一年' }));
  await userEvent.click(screen.getByRole('button', { name: '1 月' }));

  expect(screen.getByRole('button', { name: '2027 年 1 月' })).toBeVisible();
  expect(screen.queryByRole('dialog', { name: '选择月份' })).not.toBeInTheDocument();
});
