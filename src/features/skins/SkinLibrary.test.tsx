import '@testing-library/jest-dom/vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {
  deleteCompanionSkin,
  getCompanionSettings,
  importCompanionSkin,
  listCompanionSkins,
  setActiveCompanionSkin,
  subscribeToCompanionSettingsChanged,
  subscribeToCompanionSkinChanged,
  updateCompanionSkin,
  type CompanionSkin,
  type CompanionSkinState,
} from '../../lib/companion';
import { selectCompanionSkinFile } from '../../lib/desktop';
import { SkinLibrary } from './SkinLibrary';

vi.mock('../../lib/companion', async () => {
  const actual = await vi.importActual<typeof import('../../lib/companion')>('../../lib/companion');
  return {
    ...actual,
    companionAssetUrl: (path: string) => path,
    listCompanionSkins: vi.fn(),
    getCompanionSettings: vi.fn(),
    importCompanionSkin: vi.fn(),
    updateCompanionSkin: vi.fn(),
    setActiveCompanionSkin: vi.fn(),
    deleteCompanionSkin: vi.fn(),
    subscribeToCompanionSkinChanged: vi.fn(),
    subscribeToCompanionSettingsChanged: vi.fn(),
  };
});

vi.mock('../../lib/desktop', () => ({
  selectCompanionSkinFile: vi.fn(),
}));

const builtins: CompanionSkin[] = [
  skin('quiet-aurora', 'Quiet Aurora', 'builtin', 'quiet_aurora'),
  skin('porcelain-pearl', 'Porcelain Pearl', 'builtin', 'porcelain_pearl'),
  skin('deep-ink', 'Deep Ink', 'builtin', 'deep_ink'),
];
const custom = skin('skin-local', '夜航纹理', 'image', 'custom');

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listCompanionSkins).mockResolvedValue(builtins);
  vi.mocked(getCompanionSettings).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    motionEnabled: true,
    visible: true,
    placement: null,
  });
  vi.mocked(subscribeToCompanionSkinChanged).mockResolvedValue(vi.fn());
  vi.mocked(subscribeToCompanionSettingsChanged).mockResolvedValue(vi.fn());
});

it('lists all builtins and switches only after the command succeeds', async () => {
  let finishSwitch: (state: CompanionSkinState) => void = () => undefined;
  vi.mocked(setActiveCompanionSkin).mockReturnValue(new Promise((resolve) => {
    finishSwitch = resolve;
  }));
  render(<SkinLibrary />);

  expect(await screen.findByText('静谧极光')).toBeVisible();
  expect(screen.getByText('雾白珍珠')).toBeVisible();
  expect(screen.getByText('深海墨色')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: '使用深海墨色' }));
  expect(setActiveCompanionSkin).toHaveBeenCalledWith('deep-ink');
  expect(screen.getByRole('button', { name: '使用深海墨色' })).toHaveAttribute(
    'aria-pressed',
    'false',
  );

  finishSwitch({ activeSkinId: 'deep-ink', skins: builtins });
  await waitFor(() => expect(screen.getByRole('button', { name: '使用深海墨色' })).toHaveAttribute(
    'aria-pressed',
    'true',
  ));
});

it('keeps the current skin and exposes an alert when import fails', async () => {
  vi.mocked(selectCompanionSkinFile).mockResolvedValue({ path: 'broken.zip', kind: 'package' });
  vi.mocked(importCompanionSkin).mockRejectedValue(new Error('invalid package'));
  render(<SkinLibrary />);

  await screen.findByText('静谧极光');
  await userEvent.click(screen.getByRole('button', { name: '导入皮肤' }));

  expect(await screen.findByRole('alert')).toHaveTextContent('皮肤导入失败');
  expect(screen.getByRole('button', { name: '使用静谧极光' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
});

it('does nothing when the skin file dialog is cancelled', async () => {
  vi.mocked(selectCompanionSkinFile).mockResolvedValue(null);
  render(<SkinLibrary />);

  await screen.findByText('静谧极光');
  await userEvent.click(screen.getByRole('button', { name: '导入皮肤' }));

  expect(importCompanionSkin).not.toHaveBeenCalled();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});

it('offers safe controls only for local skins and saves edited values', async () => {
  vi.mocked(listCompanionSkins).mockResolvedValue([...builtins, custom]);
  vi.mocked(updateCompanionSkin).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    skins: [...builtins, { ...custom, name: '星潮' }],
  });
  render(<SkinLibrary />);

  await screen.findByText('夜航纹理');
  expect(screen.queryByRole('button', { name: '删除静谧极光' })).not.toBeInTheDocument();
  expect(screen.getByRole('slider', { name: '流动速度' })).toHaveAttribute('min', '0.5');
  expect(screen.getByRole('slider', { name: '流动速度' })).toHaveAttribute('max', '2');
  expect(screen.getByRole('slider', { name: '流光强度' })).toHaveAttribute('min', '0');
  expect(screen.getByRole('slider', { name: '流光强度' })).toHaveAttribute('max', '1');

  const nameInput = screen.getByRole('textbox', { name: '皮肤名称' });
  await userEvent.clear(nameInput);
  await userEvent.type(nameInput, '星潮');
  await userEvent.click(screen.getByRole('button', { name: '保存星潮' }));

  expect(updateCompanionSkin).toHaveBeenCalledWith(expect.objectContaining({
    id: 'skin-local',
    name: '星潮',
  }));
});

it('switches to the fallback before deleting the active local skin', async () => {
  vi.mocked(listCompanionSkins).mockResolvedValue([...builtins, custom]);
  vi.mocked(getCompanionSettings).mockResolvedValue({
    activeSkinId: custom.id,
    motionEnabled: true,
    visible: true,
    placement: null,
  });
  vi.mocked(setActiveCompanionSkin).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    skins: [...builtins, custom],
  });
  vi.mocked(deleteCompanionSkin).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    skins: builtins,
  });
  render(<SkinLibrary />);

  await screen.findByText('夜航纹理');
  await userEvent.click(screen.getByRole('button', { name: '删除夜航纹理' }));

  expect(setActiveCompanionSkin).toHaveBeenCalledWith('quiet-aurora');
  expect(deleteCompanionSkin).toHaveBeenCalledWith(custom.id);
  await waitFor(() => expect(screen.queryByText('夜航纹理')).not.toBeInTheDocument());
});

it('applies event payloads and cleans up a delayed listener registration', async () => {
  let onChanged: ((state: CompanionSkinState) => void) | undefined;
  let finishListening: ((unlisten: () => void) => void) | undefined;
  const unlisten = vi.fn();
  vi.mocked(subscribeToCompanionSkinChanged).mockImplementation((handler) => {
    onChanged = handler;
    return new Promise((resolve) => {
      finishListening = resolve;
    });
  });
  const view = render(<SkinLibrary />);

  await screen.findByText('静谧极光');
  act(() => {
    onChanged?.({ activeSkinId: custom.id, skins: [...builtins, custom] });
  });
  expect(await screen.findByText('夜航纹理')).toBeVisible();

  view.unmount();
  await act(async () => {
    finishListening?.(unlisten);
  });
  await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
});

function skin(
  id: string,
  name: string,
  source: CompanionSkin['source'],
  visualPreset: CompanionSkin['visualPreset'],
): CompanionSkin {
  return {
    id,
    name,
    source,
    visualPreset,
    texturePath: source === 'builtin' ? null : `C:\\skins\\${id}\\texture.png`,
    previewPath: source === 'builtin' ? null : `C:\\skins\\${id}\\preview.png`,
    flowColors: ['#BDA7FF', '#55D8CF'],
    flowSpeed: 1,
    flowIntensity: 0.7,
    createdAt: '2026-07-31T00:00:00Z',
  };
}
