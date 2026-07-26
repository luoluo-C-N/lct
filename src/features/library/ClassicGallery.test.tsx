import '@testing-library/jest-dom/vitest';
import { invoke } from '@tauri-apps/api/core';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../../App';
import type { Asset } from '../../lib/assets';
import { ClassicGallery } from './ClassicGallery';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
  invoke: vi.fn(),
}));

function asset(id: string, createdAt: string): Asset {
  return {
    id,
    createdAt,
    importedAt: createdAt,
    source: 'import',
    originalPath: `C:\\images\\${id}.png`,
    previewPath: `C:\\previews\\${id}.jpg`,
    albumId: null,
    favorite: false,
    syncVersion: 1,
  };
}

it('renders every asset returned for the initial month', async () => {
  render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([
        asset('sunset', '2026-07-25T12:00:00Z'),
        asset('forest', '2026-07-24T12:00:00Z'),
      ])}
    />,
  );

  const sunset = await screen.findByRole('img', { name: 'sunset' });
  expect(sunset).toBeVisible();
  expect(sunset).toHaveAttribute('src', 'asset://localhost/C:\\previews\\sunset.jpg');
  expect(screen.getByRole('img', { name: 'forest' })).toBeVisible();
});

it('renders a retry action after a month query fails', async () => {
  const loadMonth = vi.fn()
    .mockRejectedValueOnce(new Error('offline'))
    .mockResolvedValueOnce([asset('recovered', '2026-07-25T12:00:00Z')]);
  render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={loadMonth}
    />,
  );

  expect(await screen.findByText('无法加载本月图片')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: '重试' }));

  expect(await screen.findByRole('img', { name: 'recovered' })).toBeVisible();
  expect(screen.queryByText('无法加载本月图片')).not.toBeInTheDocument();
  expect(loadMonth).toHaveBeenCalledTimes(2);
});

it('shows an empty state when the initial month has no assets', async () => {
  render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([])}
    />,
  );

  expect(await screen.findByText('这个月还没有影像')).toBeVisible();
});

it('renders queried assets after switching the application to gallery mode', async () => {
  vi.mocked(invoke).mockResolvedValue([
    asset('gallery-memory', '2026-07-25T12:00:00Z'),
  ]);
  render(<App />);

  await userEvent.click(screen.getByRole('button', { name: '图库' }));

  expect(await screen.findByRole('img', { name: 'gallery-memory' })).toBeVisible();
});
