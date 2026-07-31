import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { useCompanionPosition, type CompanionWindowApi } from './useCompanionPosition';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

it('debounces moved events and saves logical placement after 250ms', async () => {
  let onMoved: ((event: { payload: { x: number; y: number } }) => void) | undefined;
  const savePlacement = vi.fn().mockResolvedValue(undefined);
  const windowApi = {
    onMoved: vi.fn(async (handler) => {
      onMoved = handler;
      return vi.fn();
    }),
    outerSize: vi.fn().mockResolvedValue({ width: 464, height: 640 }),
    scaleFactor: vi.fn().mockResolvedValue(2),
  } as unknown as CompanionWindowApi;
  renderHook(() => useCompanionPosition(windowApi, savePlacement));
  await act(async () => Promise.resolve());

  act(() => {
    onMoved?.({ payload: { x: 100, y: 200 } });
    vi.advanceTimersByTime(249);
  });
  expect(savePlacement).not.toHaveBeenCalled();

  await act(async () => {
    vi.advanceTimersByTime(1);
    await Promise.resolve();
  });
  expect(savePlacement).toHaveBeenCalledWith({
    x: 50,
    y: 100,
    width: 232,
    height: 320,
  });
});

it('cleans up a listener that finishes registering after unmount', async () => {
  let finishListening: ((unlisten: () => void) => void) | undefined;
  const unlisten = vi.fn();
  const windowApi = {
    onMoved: vi.fn(() => new Promise((resolve) => {
      finishListening = resolve;
    })),
  } as unknown as CompanionWindowApi;
  const view = renderHook(() => useCompanionPosition(windowApi, vi.fn()));

  view.unmount();
  await act(async () => {
    finishListening?.(unlisten);
  });

  expect(unlisten).toHaveBeenCalledTimes(1);
});

it('does not register movement persistence while native expansion is active', () => {
  const windowApi = {
    onMoved: vi.fn(),
  } as unknown as CompanionWindowApi;

  renderHook(() => useCompanionPosition(windowApi, vi.fn(), false));

  expect(windowApi.onMoved).not.toHaveBeenCalled();
});
