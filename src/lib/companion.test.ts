import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  beginCompanionRegionSelection,
  finishCompanionRegionSelection,
  importCompanionSkin,
  listCompanionSkins,
  setActiveCompanionSkin,
  subscribeToCompanionSettingsChanged,
  subscribeToCompanionSkinChanged,
  subscribeToCompanionVisibilityChanged,
  type CompanionSettings,
  type CompanionSkinState,
} from './companion';
import { capture, selectCompanionSkinFile } from './desktop';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

describe('companion IPC adapter', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('invokes package and image imports through the same validated command', async () => {
    await importCompanionSkin('C:\\skins\\theme.zip');
    await importCompanionSkin('C:\\skins\\texture.png');

    expect(invoke).toHaveBeenNthCalledWith(1, 'import_companion_skin', {
      path: 'C:\\skins\\theme.zip',
    });
    expect(invoke).toHaveBeenNthCalledWith(2, 'import_companion_skin', {
      path: 'C:\\skins\\texture.png',
    });
  });

  it('uses exact list and active-skin command arguments', async () => {
    await listCompanionSkins();
    await setActiveCompanionSkin('deep-ink');

    expect(invoke).toHaveBeenNthCalledWith(1, 'list_companion_skins');
    expect(invoke).toHaveBeenNthCalledWith(2, 'set_active_companion_skin', {
      skinId: 'deep-ink',
    });
  });

  it('forwards the selected region and native selection session commands', async () => {
    await beginCompanionRegionSelection();
    await finishCompanionRegionSelection();
    await capture('region', { x: 30, y: 45, width: 120, height: 75 });

    expect(invoke).toHaveBeenNthCalledWith(1, 'begin_companion_region_selection');
    expect(invoke).toHaveBeenNthCalledWith(2, 'finish_companion_region_selection');
    expect(invoke).toHaveBeenNthCalledWith(3, 'capture', {
      mode: 'region',
      region: { x: 30, y: 45, width: 120, height: 75 },
    });
  });

  it('forwards complete event payloads and returns each unlisten function', async () => {
    const unlistenSkin = vi.fn();
    const unlistenSettings = vi.fn();
    const unlistenVisibility = vi.fn();
    const skinState = { activeSkinId: 'deep-ink', skins: [] } satisfies CompanionSkinState;
    const settings = {
      activeSkinId: 'deep-ink',
      motionEnabled: true,
      visible: false,
      placement: null,
    } satisfies CompanionSettings;
    vi.mocked(listen)
      .mockImplementationOnce(async (_event, handler) => {
        handler({ payload: skinState } as never);
        return unlistenSkin;
      })
      .mockImplementationOnce(async (_event, handler) => {
        handler({ payload: settings } as never);
        return unlistenSettings;
      })
      .mockImplementationOnce(async (_event, handler) => {
        handler({ payload: settings } as never);
        return unlistenVisibility;
      });
    const onSkin = vi.fn();
    const onSettings = vi.fn();
    const onVisibility = vi.fn();

    await expect(subscribeToCompanionSkinChanged(onSkin)).resolves.toBe(unlistenSkin);
    await expect(subscribeToCompanionSettingsChanged(onSettings)).resolves.toBe(unlistenSettings);
    await expect(subscribeToCompanionVisibilityChanged(onVisibility)).resolves.toBe(
      unlistenVisibility,
    );
    expect(onSkin).toHaveBeenCalledWith(skinState);
    expect(onSettings).toHaveBeenCalledWith(settings);
    expect(onVisibility).toHaveBeenCalledWith(settings);
  });

  it('selects only supported companion skin files', async () => {
    vi.mocked(open).mockResolvedValue('C:\\skins\\theme.zip');

    await expect(selectCompanionSkinFile()).resolves.toEqual({
      path: 'C:\\skins\\theme.zip',
      kind: 'package',
    });
    expect(open).toHaveBeenCalledWith({
      multiple: false,
      directory: false,
      filters: [{ name: '皮肤文件', extensions: ['png', 'webp', 'zip'] }],
    });
  });
});
