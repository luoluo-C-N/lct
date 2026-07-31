import '@testing-library/jest-dom/vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {
  beginCompanionRegionSelection,
  finishCompanionRegionSelection,
  focusMainWindow,
  getCompanionSettings,
  hideCompanion,
  listCompanionSkins,
  setActiveCompanionSkin,
  setCompanionExpanded,
  subscribeToCompanionSettingsChanged,
  subscribeToCompanionSkinChanged,
  type CompanionSkin,
} from '../../lib/companion';
import { capture, importFiles, selectImageFiles } from '../../lib/desktop';
import { CompanionWindow } from './CompanionWindow';
import type { CompanionWindowApi } from './useCompanionPosition';

vi.mock('../../lib/companion', async () => {
  const actual = await vi.importActual<typeof import('../../lib/companion')>('../../lib/companion');
  return {
    ...actual,
    companionAssetUrl: (path: string) => path,
    listCompanionSkins: vi.fn(),
    getCompanionSettings: vi.fn(),
    setActiveCompanionSkin: vi.fn(),
    setCompanionExpanded: vi.fn(),
    focusMainWindow: vi.fn(),
    hideCompanion: vi.fn(),
    beginCompanionRegionSelection: vi.fn(),
    finishCompanionRegionSelection: vi.fn(),
    subscribeToCompanionSkinChanged: vi.fn(),
    subscribeToCompanionSettingsChanged: vi.fn(),
    saveCompanionPlacement: vi.fn(),
  };
});

vi.mock('../../lib/desktop', () => ({
  capture: vi.fn(),
  importFiles: vi.fn(),
  selectImageFiles: vi.fn(),
}));

const skins: CompanionSkin[] = [
  skin('quiet-aurora', 'quiet_aurora'),
  skin('porcelain-pearl', 'porcelain_pearl'),
  skin('deep-ink', 'deep_ink'),
];
let windowApi: CompanionWindowApi;

beforeEach(() => {
  vi.clearAllMocks();
  windowApi = {
    startDragging: vi.fn().mockResolvedValue(undefined),
    onMoved: vi.fn().mockResolvedValue(vi.fn()),
    outerSize: vi.fn().mockResolvedValue({ width: 72, height: 72 }),
    scaleFactor: vi.fn().mockResolvedValue(1),
  };
  vi.mocked(listCompanionSkins).mockResolvedValue(skins);
  vi.mocked(getCompanionSettings).mockResolvedValue({
    activeSkinId: 'quiet-aurora',
    motionEnabled: true,
    visible: true,
    placement: null,
  });
  vi.mocked(setCompanionExpanded).mockResolvedValue(undefined);
  vi.mocked(focusMainWindow).mockResolvedValue(undefined);
  vi.mocked(hideCompanion).mockResolvedValue({
    activeSkinId: 'quiet-aurora', motionEnabled: true, visible: false, placement: null,
  });
  vi.mocked(beginCompanionRegionSelection).mockResolvedValue({ scaleFactor: 2 });
  vi.mocked(finishCompanionRegionSelection).mockResolvedValue(undefined);
  vi.mocked(subscribeToCompanionSkinChanged).mockResolvedValue(vi.fn());
  vi.mocked(subscribeToCompanionSettingsChanged).mockResolvedValue(vi.fn());
  vi.mocked(capture).mockResolvedValue(undefined);
  vi.mocked(importFiles).mockResolvedValue([]);
  vi.mocked(selectImageFiles).mockResolvedValue([]);
});

it('expands, restores focus on Escape, and requests anchored native resize', async () => {
  renderCompanion();
  const trigger = screen.getByRole('button', { name: '打开悬浮助手菜单' });

  await userEvent.click(trigger);
  expect(windowApi.startDragging).not.toHaveBeenCalled();
  expect(setCompanionExpanded).toHaveBeenCalledWith(true);
  expect(trigger).toHaveAttribute('aria-expanded', 'true');

  await userEvent.keyboard('{Escape}');
  expect(setCompanionExpanded).toHaveBeenLastCalledWith(false);
  expect(trigger).toHaveFocus();
});

it('starts native dragging only from the orb drag handle', async () => {
  renderCompanion();
  await act(async () => Promise.resolve());

  const handle = screen.getByTestId('companion-drag-handle');
  fireEvent(handle, pointerEvent('pointerdown', { button: 1, clientX: 10, clientY: 10 }));
  expect(windowApi.startDragging).not.toHaveBeenCalled();
  fireEvent(handle, pointerEvent('pointerdown', { button: 0, clientX: 10, clientY: 10 }));
  expect(windowApi.startDragging).not.toHaveBeenCalled();
  fireEvent(handle, pointerEvent('pointermove', { buttons: 1, clientX: 20, clientY: 10 }));
  expect(windowApi.startDragging).toHaveBeenCalledTimes(1);
});

it('keeps the orb on the nearest left and top edges while expanded', async () => {
  Object.assign(windowApi, {
    outerPosition: vi.fn().mockResolvedValue({ x: -1200, y: 20 }),
    currentMonitor: vi.fn().mockResolvedValue({
      workArea: {
        position: { x: -1280, y: 0 },
        size: { width: 1280, height: 720 },
      },
    }),
  });
  renderCompanion();

  await openMenu();

  expect(screen.getByRole('complementary', { name: '悬浮助手' }))
    .toHaveClass('companion--anchor-left', 'companion--anchor-top');
});

it('quick-switches skins only after the command succeeds', async () => {
  vi.mocked(setActiveCompanionSkin).mockResolvedValue({
    activeSkinId: 'deep-ink',
    skins,
  });
  renderCompanion();
  await openMenu();

  await userEvent.click(await screen.findByRole('button', { name: '切换到深海墨色' }));

  expect(setActiveCompanionSkin).toHaveBeenCalledWith('deep-ink');
  await waitFor(() => expect(screen.getByTestId('companion-orb')).toHaveClass('companion-orb--ink'));
});

it('opens the main library and hides the companion from menu actions', async () => {
  renderCompanion();
  await openMenu();

  await userEvent.click(screen.getByRole('button', { name: '打开影像库' }));
  await userEvent.click(screen.getByRole('button', { name: '隐藏悬浮助手' }));

  expect(focusMainWindow).toHaveBeenCalledTimes(1);
  expect(hideCompanion).toHaveBeenCalledTimes(1);
});

it('imports every selected image and reports completion', async () => {
  vi.mocked(selectImageFiles).mockResolvedValue(['C:\\images\\first.png', 'C:\\images\\second.jpg']);
  vi.mocked(importFiles).mockResolvedValue([{} as never, {} as never]);
  renderCompanion();
  await openMenu();

  await userEvent.click(screen.getByRole('button', { name: '导入图片' }));

  expect(await screen.findByText('已导入 2 张图片')).toBeVisible();
  expect(importFiles).toHaveBeenCalledWith(['C:\\images\\first.png', 'C:\\images\\second.jpg']);
});

it('keeps capture and import failures recoverable', async () => {
  vi.mocked(capture).mockRejectedValueOnce(new Error('capture unavailable'));
  renderCompanion();
  await openMenu();
  await userEvent.click(screen.getByRole('button', { name: '窗口截图' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('截图失败，请重试。');

  vi.mocked(selectImageFiles).mockResolvedValue(['C:\\images\\broken.png']);
  vi.mocked(importFiles).mockRejectedValueOnce(new Error('import unavailable'));
  await userEvent.click(screen.getByRole('button', { name: '导入图片' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('导入失败，请重新选择图片。');
});

it('selects a DPI-correct region before invoking region capture', async () => {
  renderCompanion();
  await openMenu();

  await userEvent.click(screen.getByRole('button', { name: '区域截图' }));
  const overlay = await screen.findByRole('dialog', { name: '选择截图区域' });
  fireEvent(overlay, pointerEvent('pointerdown', { button: 0, clientX: 10, clientY: 15 }));
  fireEvent(overlay, pointerEvent('pointermove', { buttons: 1, clientX: 50, clientY: 55 }));
  fireEvent(overlay, pointerEvent('pointerup', { button: 0, clientX: 50, clientY: 55 }));

  await waitFor(() => expect(finishCompanionRegionSelection).toHaveBeenCalledTimes(1));
  expect(capture).toHaveBeenCalledWith('region', { x: 20, y: 30, width: 80, height: 80 });
});

function renderCompanion() {
  return render(<CompanionWindow windowApi={windowApi} />);
}

async function openMenu() {
  await userEvent.click(screen.getByRole('button', { name: '打开悬浮助手菜单' }));
}

function skin(id: string, visualPreset: CompanionSkin['visualPreset']): CompanionSkin {
  return {
    id,
    name: id,
    source: 'builtin',
    visualPreset,
    texturePath: null,
    previewPath: null,
    flowColors: ['#BD9FFF', '#FFF4DC'],
    flowSpeed: 1,
    flowIntensity: 0.7,
    createdAt: '2026-07-31T00:00:00Z',
  };
}

function pointerEvent(type: string, properties: Record<string, number>) {
  const event = new Event(type, { bubbles: true });
  Object.entries(properties).forEach(([name, value]) => {
    Object.defineProperty(event, name, { value });
  });
  return event;
}
