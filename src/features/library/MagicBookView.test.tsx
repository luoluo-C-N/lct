import '@testing-library/jest-dom/vitest';
import { listen, type Event } from '@tauri-apps/api/event';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { Asset } from '../../lib/assets';
import { MagicBookView } from './MagicBookView';

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
    albumId: null,
    favorite: false,
    syncVersion: 1,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });

  return { promise, resolve };
}

it('replaces date flow and image cards with the selected month response', async () => {
  const loadMonth = vi.fn()
    .mockResolvedValueOnce([asset('july', '2026-07-25T12:00:00Z')])
    .mockResolvedValueOnce([asset('january', '2026-01-18T12:00:00Z')]);
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} loadMonth={loadMonth} />);

  await userEvent.click(await screen.findByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '1 月' }));

  expect(await screen.findByRole('img', { name: 'january' })).toBeVisible();
  expect(screen.queryByRole('img', { name: 'july' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: /01 \/ 18/ })).toBeVisible();
  expect(screen.queryByRole('button', { name: /07 \/ 25/ })).not.toBeInTheDocument();
});

it('shows no slider for exactly one selected-day asset', async () => {
  render(
    <MagicBookView
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([asset('only', '2026-07-25T12:00:00Z')])}
    />,
  );

  expect(await screen.findByRole('img', { name: 'only' })).toBeVisible();
  expect(screen.queryByRole('slider', { name: '图片浏览滑轨' })).not.toBeInTheDocument();
});

it('shows a slider for multiple assets on the selected day', async () => {
  render(
    <MagicBookView
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([
        asset('first', '2026-07-25T12:00:00Z'),
        asset('second', '2026-07-25T13:00:00Z'),
      ])}
    />,
  );

  expect(await screen.findByRole('img', { name: 'first' })).toBeVisible();
  expect(screen.getByRole('slider', { name: '图片浏览滑轨' })).toHaveAttribute('max', '1');
});

it('clears the previous month while the next month is loading', async () => {
  const mayAssets = deferred<Asset[]>();
  const loadMonth = vi.fn()
    .mockResolvedValueOnce([asset('july', '2026-07-25T12:00:00Z')])
    .mockReturnValueOnce(mayAssets.promise);
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} loadMonth={loadMonth} />);

  await userEvent.click(await screen.findByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '5 月' }));

  expect(screen.getByText('正在翻阅影像…')).toBeVisible();
  expect(screen.queryByRole('img', { name: 'july' })).not.toBeInTheDocument();

  await act(async () => {
    mayAssets.resolve([asset('may', '2026-05-18T12:00:00Z')]);
  });
  expect(await screen.findByRole('img', { name: 'may' })).toBeVisible();
});

it('shows a Chinese empty state when a month has no assets', async () => {
  render(
    <MagicBookView
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([])}
    />,
  );

  expect(await screen.findByText('这个月还没有影像')).toBeVisible();
});

it('shows a failure state and retries the month request', async () => {
  const loadMonth = vi.fn()
    .mockRejectedValueOnce(new Error('repository unavailable'))
    .mockResolvedValueOnce([asset('recovered', '2026-07-25T12:00:00Z')]);
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} loadMonth={loadMonth} />);

  expect(await screen.findByText('影像加载失败')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: '重试' }));

  expect(await screen.findByRole('img', { name: 'recovered' })).toBeVisible();
  expect(screen.queryByText('影像加载失败')).not.toBeInTheDocument();
});

it('replaces the selected date assets with the day response', async () => {
  const loadDay = vi.fn().mockResolvedValue([
    asset('refreshed', '2026-07-24T15:00:00Z'),
  ]);
  render(
    <MagicBookView
      initialMonth={{ year: 2026, month: 7 }}
      loadMonth={() => Promise.resolve([
        asset('newest', '2026-07-25T12:00:00Z'),
        asset('cached', '2026-07-24T12:00:00Z'),
      ])}
      loadDay={loadDay}
    />,
  );

  await userEvent.click(await screen.findByRole('button', { name: /07 \/ 24/ }));

  expect(await screen.findByRole('img', { name: 'refreshed' })).toBeVisible();
  expect(screen.queryByRole('img', { name: 'cached' })).not.toBeInTheDocument();
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
    <MagicBookView
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
