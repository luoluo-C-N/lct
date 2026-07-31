import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

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
