import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn(),
  invoke: vi.fn().mockResolvedValue([]),
}));

it('renders the library shell', async () => {
  render(<App />);

  expect(screen.getByRole('main', { name: '影像资料库' })).toBeVisible();
  expect(await screen.findByText('这个月还没有影像')).toBeVisible();
});
