import '@testing-library/jest-dom/vitest';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { CompanionWindow } from './CompanionWindow';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

it('opens capture actions', async () => {
  render(<CompanionWindow />);
  await userEvent.click(screen.getByRole('button', { name: '悬浮角色' }));
  expect(screen.getByRole('button', { name: '区域截图' })).toBeTruthy();
});

it('imports every image selected from the companion menu', async () => {
  vi.mocked(open).mockResolvedValue([
    'C:\\images\\first.png',
    'C:\\images\\second.jpg',
  ]);
  vi.mocked(invoke).mockResolvedValue([{}, {}]);
  render(<CompanionWindow />);

  await userEvent.click(screen.getByRole('button', { name: '悬浮角色' }));
  await userEvent.click(screen.getByRole('button', { name: '导入图片' }));

  expect(await screen.findByText('已导入 2 张图片')).toBeVisible();
  expect(invoke).toHaveBeenCalledWith('import_files', {
    paths: ['C:\\images\\first.png', 'C:\\images\\second.jpg'],
  });
});

it('shows a recoverable alert when capture fails', async () => {
  vi.mocked(invoke).mockRejectedValue(new Error('capture unavailable'));
  render(<CompanionWindow />);

  await userEvent.click(screen.getByRole('button', { name: '悬浮角色' }));
  await userEvent.click(screen.getByRole('button', { name: '区域截图' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('截图失败，请重试。');
  expect(screen.getByRole('button', { name: '区域截图' })).toBeVisible();
});
