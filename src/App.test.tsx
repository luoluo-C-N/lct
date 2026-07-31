import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {
  getCompanionSettings,
  showCompanion,
  subscribeToCompanionVisibilityChanged,
} from './lib/companion';
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

vi.mock('./lib/companion', () => ({
  getCompanionSettings: vi.fn(),
  showCompanion: vi.fn(),
  hideCompanion: vi.fn(),
  subscribeToCompanionVisibilityChanged: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(getCompanionSettings).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    motionEnabled: true,
    visible: false,
    placement: null,
  });
  vi.mocked(showCompanion).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    motionEnabled: true,
    visible: true,
    placement: null,
  });
  vi.mocked(subscribeToCompanionVisibilityChanged).mockResolvedValue(vi.fn());
});

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

it('restores a hidden companion from the main window', async () => {
  render(<App />);

  await userEvent.click(await screen.findByRole('button', { name: '显示悬浮助手' }));

  expect(showCompanion).toHaveBeenCalledTimes(1);
  expect(await screen.findByRole('button', { name: '隐藏悬浮助手' })).toBeVisible();
});

it('keeps the visibility state and exposes an alert when restoring fails', async () => {
  vi.mocked(showCompanion).mockRejectedValue(new Error('window unavailable'));
  render(<App />);

  await userEvent.click(await screen.findByRole('button', { name: '显示悬浮助手' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('悬浮助手状态更新失败');
  expect(screen.getByRole('button', { name: '显示悬浮助手' })).toBeVisible();
});
