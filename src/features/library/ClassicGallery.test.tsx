import '@testing-library/jest-dom/vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen, type Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../../App';
import type { Asset } from '../../lib/assets';
import { ClassicGallery } from './ClassicGallery';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://localhost/${path}`,
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listen).mockResolvedValue(vi.fn());
});

function asset(id: string, createdAt: string): Asset {
  return {
    id,
    createdAt,
    importedAt: createdAt,
    source: 'import',
    originalPath: `C:\\images\\${id}.png`,
    previewPath: `C:\\previews\\${id}.jpg`,
    displayName: `${id}.png`,
    albumId: null,
    tags: [],
    favorite: false,
    deletedAt: null,
    captureMode: null,
    annotationData: null,
    syncVersion: 1,
    cloudId: null,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });

  return { promise, resolve };
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

it('replaces gallery assets after selecting another year and month', async () => {
  const loadMonth = vi.fn()
    .mockResolvedValueOnce([asset('july', '2026-07-25T12:00:00Z')])
    .mockResolvedValueOnce([asset('january', '2027-01-18T12:00:00Z')]);
  render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={loadMonth}
    />,
  );

  expect(await screen.findByRole('img', { name: 'july' })).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '下一年' }));
  await userEvent.click(screen.getByRole('button', { name: '1 月' }));

  expect(await screen.findByRole('img', { name: 'january' })).toBeVisible();
  expect(screen.queryByRole('img', { name: 'july' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: '2027 年 1 月' })).toBeVisible();
});

it('renders queried assets after switching the application to gallery mode', async () => {
  vi.mocked(invoke).mockResolvedValue([
    asset('gallery-memory', '2026-07-25T12:00:00Z'),
  ]);
  render(<App />);

  await userEvent.click(screen.getByRole('button', { name: '图库' }));

  expect(await screen.findByRole('img', { name: 'gallery-memory' })).toBeVisible();
});

it('initializes both application views with the current local month', async () => {
  vi.mocked(invoke).mockResolvedValue([]);
  render(<App now={() => new Date(2031, 1, 14, 12)} />);

  await waitFor(() => {
    expect(invoke).toHaveBeenCalledWith(
      'list_assets_by_month',
      { year: 2031, month: 2 },
    );
  });

  await userEvent.click(screen.getByRole('button', { name: '图库' }));

  await waitFor(() => {
    const monthLoads = vi.mocked(invoke).mock.calls.filter(([command, args]) => {
      const monthArgs = args as { year?: number; month?: number } | undefined;
      return command === 'list_assets_by_month'
        && monthArgs?.year === 2031
        && monthArgs.month === 2;
    });
    expect(monthLoads).toHaveLength(2);
  });
});

it('reloads only current-month asset events and unsubscribes on cleanup', async () => {
  let assetCreated: ((event: Event<Asset>) => void) | undefined;
  const unlisten = vi.fn();
  vi.mocked(listen).mockImplementation(async (_event, handler) => {
    assetCreated = handler as (event: Event<Asset>) => void;
    return unlisten;
  });
  const loadMonth = vi.fn()
    .mockResolvedValueOnce([asset('original', '2026-07-24T12:00:00Z')])
    .mockResolvedValueOnce([asset('current-month', '2026-07-26T12:00:00Z')]);
  const { unmount } = render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={loadMonth}
    />,
  );

  expect(await screen.findByRole('img', { name: 'original' })).toBeVisible();
  await waitFor(() => expect(assetCreated).toBeDefined());

  act(() => {
    assetCreated?.({
      event: 'asset-created',
      id: 1,
      payload: asset('other-month', '2026-08-01T12:00:00Z'),
    });
  });
  expect(screen.getByRole('img', { name: 'original' })).toBeVisible();
  expect(loadMonth).toHaveBeenCalledTimes(1);

  act(() => {
    assetCreated?.({
      event: 'asset-created',
      id: 2,
      payload: asset('event-asset', '2026-07-26T12:00:00Z'),
    });
  });
  expect(await screen.findByRole('img', { name: 'current-month' })).toBeVisible();
  expect(screen.queryByRole('img', { name: 'original' })).not.toBeInTheDocument();

  unmount();
  expect(unlisten).toHaveBeenCalledOnce();
});

it('cleans delayed listener registrations after a month change and unmount', async () => {
  const firstRegistration = deferred<() => void>();
  const secondRegistration = deferred<() => void>();
  const firstUnlisten = vi.fn();
  const secondUnlisten = vi.fn();
  vi.mocked(listen)
    .mockReturnValueOnce(firstRegistration.promise)
    .mockReturnValueOnce(secondRegistration.promise);
  const { unmount } = render(
    <ClassicGallery
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([])}
    />,
  );

  expect(await screen.findByText('这个月还没有影像')).toBeVisible();
  await waitFor(() => expect(listen).toHaveBeenCalledTimes(1));

  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '1 月' }));
  await waitFor(() => expect(listen).toHaveBeenCalledTimes(2));

  await act(async () => {
    firstRegistration.resolve(firstUnlisten);
  });
  expect(firstUnlisten).toHaveBeenCalledOnce();

  unmount();
  await act(async () => {
    secondRegistration.resolve(secondUnlisten);
  });
  expect(secondUnlisten).toHaveBeenCalledOnce();
});
