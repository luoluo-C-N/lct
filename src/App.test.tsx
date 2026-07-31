import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';

vi.mock('./features/skins/SkinLibrary', () => ({
  SkinLibrary: () => <section aria-label="皮肤库">皮肤管理</section>,
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn(),
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

it('renders the library shell', async () => {
  render(<App />);

  expect(screen.getByRole('main', { name: '影像资料库' })).toBeVisible();
  expect(screen.queryByRole('complementary', { name: '悬浮助手' })).not.toBeInTheDocument();
  expect(await screen.findByText('这个月还没有影像')).toBeVisible();
});

it('opens the skin library as a main-window view', async () => {
  render(<App />);

  await userEvent.click(screen.getByRole('button', { name: '皮肤库' }));

  expect(screen.getByRole('region', { name: '皮肤库' })).toBeVisible();
  expect(screen.queryByRole('main', { name: '魔法书资料库' })).not.toBeInTheDocument();
});
