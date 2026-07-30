import '@testing-library/jest-dom/vitest';
import { useState } from 'react';
import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterAll, beforeEach, vi } from 'vitest';
import { MonthPicker } from './MonthPicker';

const showModal = vi.spyOn(HTMLDialogElement.prototype, 'showModal');

beforeEach(() => {
  showModal.mockClear();
});

afterAll(() => {
  showModal.mockRestore();
});

function MonthPickerHarness() {
  const [month, setMonth] = useState({ year: 2026, month: 7 });
  return <MonthPicker month={month} onSelect={setMonth} />;
}

it('opens a labelled native modal and moves focus inside it', async () => {
  render(<MonthPickerHarness />);
  const trigger = screen.getByRole('button', { name: '2026 年 7 月' });

  expect(trigger).toHaveAttribute('aria-haspopup', 'dialog');
  expect(trigger).toHaveAttribute('aria-expanded', 'false');

  await userEvent.click(trigger);

  const dialog = screen.getByRole('dialog', { name: '选择月份' });
  expect(showModal).toHaveBeenCalledOnce();
  expect(trigger).toHaveAttribute('aria-expanded', 'true');
  expect(trigger).toHaveAttribute('aria-controls', dialog.id);
  expect(dialog).toContainElement(document.activeElement as HTMLElement);
});

it('closes on Escape and restores focus to the trigger', async () => {
  render(<MonthPickerHarness />);
  const trigger = screen.getByRole('button', { name: '2026 年 7 月' });
  await userEvent.click(trigger);
  await userEvent.click(screen.getByRole('button', { name: '下一年' }));

  await userEvent.keyboard('{Escape}');

  expect(screen.queryByRole('dialog', { name: '选择月份' })).not.toBeInTheDocument();
  expect(trigger).toHaveAttribute('aria-expanded', 'false');
  expect(trigger).toHaveFocus();
});

it('closes on a native cancel event and restores focus to the trigger', async () => {
  render(<MonthPickerHarness />);
  const trigger = screen.getByRole('button', { name: '2026 年 7 月' });
  await userEvent.click(trigger);
  const dialog = screen.getByRole('dialog', { name: '选择月份' });
  await userEvent.click(screen.getByRole('button', { name: '下一年' }));

  fireEvent(dialog, new Event('cancel', { cancelable: true }));

  expect(screen.queryByRole('dialog', { name: '选择月份' })).not.toBeInTheDocument();
  expect(trigger).toHaveFocus();
});

it('selects a month from the next year and closes the album shelf', async () => {
  render(<MonthPickerHarness />);

  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '下一年' }));
  await userEvent.click(screen.getByRole('button', { name: '1 月' }));

  expect(screen.getByRole('button', { name: '2027 年 1 月' })).toBeVisible();
  expect(screen.queryByRole('dialog', { name: '选择月份' })).not.toBeInTheDocument();
});

it('renders all month volumes outside an overflow-clipped trigger container', async () => {
  const { container } = render(
    <aside style={{ overflow: 'hidden', width: 220 }}>
      <MonthPicker month={{ year: 2026, month: 7 }} onSelect={() => undefined} />
    </aside>,
  );

  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));

  const shelf = screen.getByRole('dialog', { name: '选择月份' });
  expect(shelf.parentElement).toBe(document.body);
  expect(container.querySelector('dialog')).not.toBeInTheDocument();
  expect(within(shelf).getAllByRole('button', { name: /^\d+ 月$/ })).toHaveLength(12);
});
