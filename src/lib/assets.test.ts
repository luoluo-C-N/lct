import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { assetPreviewUrl, listAssetsByDay, listAssetsByMonth } from './assets';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: vi.fn(),
  invoke: vi.fn(),
}));

describe('asset IPC adapter', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('requests assets for the selected month', async () => {
    await listAssetsByMonth(2026, 7);

    expect(invoke).toHaveBeenCalledWith('list_assets_by_month', { year: 2026, month: 7 });
  });

  it('requests assets for the selected day', async () => {
    await listAssetsByDay(2026, 7, 26);

    expect(invoke).toHaveBeenCalledWith('list_assets_by_day', { year: 2026, month: 7, day: 26 });
  });

  it('converts a stored preview path into a webview URL', () => {
    vi.mocked(convertFileSrc).mockReturnValue('asset://localhost/C:/cache/preview.png');

    expect(assetPreviewUrl('C:/cache/preview.png')).toBe('asset://localhost/C:/cache/preview.png');
    expect(convertFileSrc).toHaveBeenCalledWith('C:/cache/preview.png');
  });
});
