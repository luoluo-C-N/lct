import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type VisualPreset =
  | 'quiet_aurora'
  | 'porcelain_pearl'
  | 'deep_ink'
  | 'custom';

export type CompanionSkin = {
  id: string;
  name: string;
  source: 'builtin' | 'image' | 'package';
  visualPreset: VisualPreset;
  texturePath: string | null;
  previewPath: string | null;
  flowColors: string[];
  flowSpeed: number;
  flowIntensity: number;
  createdAt: string;
};

export type WindowPlacement = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type CompanionSettings = {
  activeSkinId: string;
  motionEnabled: boolean;
  visible: boolean;
  placement: WindowPlacement | null;
};

export type CompanionSkinState = {
  activeSkinId: string;
  skins: CompanionSkin[];
};

export type SkinImportSelection = {
  path: string;
  kind: 'image' | 'package';
};

export const listCompanionSkins = () =>
  invoke<CompanionSkin[]>('list_companion_skins');

export const getCompanionSettings = () =>
  invoke<CompanionSettings>('get_companion_settings');

export const importCompanionSkin = (path: string) =>
  invoke<CompanionSkin>('import_companion_skin', { path });

export const updateCompanionSkin = (skin: CompanionSkin) =>
  invoke<CompanionSkinState>('update_companion_skin', { skin });

export const setActiveCompanionSkin = (skinId: string) =>
  invoke<CompanionSkinState>('set_active_companion_skin', { skinId });

export const deleteCompanionSkin = (skinId: string) =>
  invoke<CompanionSkinState>('delete_companion_skin', { skinId });

export const showCompanion = () =>
  invoke<CompanionSettings>('show_companion');

export const hideCompanion = () =>
  invoke<CompanionSettings>('hide_companion');

export const focusMainWindow = () => invoke<void>('focus_main_window');

export const setCompanionExpanded = (expanded: boolean) =>
  invoke<void>('set_companion_expanded', { expanded });

export const beginCompanionRegionSelection = () =>
  invoke<{ scaleFactor: number }>('begin_companion_region_selection');

export const finishCompanionRegionSelection = () =>
  invoke<void>('finish_companion_region_selection');

export const saveCompanionPlacement = (placement: WindowPlacement) =>
  invoke<CompanionSettings>('save_companion_placement', { placement });

export const companionAssetUrl = (path: string) => convertFileSrc(path);

export const subscribeToCompanionSkinChanged = (
  onChanged: (state: CompanionSkinState) => void,
) => listen<CompanionSkinState>('companion-skin-changed', (event) => onChanged(event.payload));

export const subscribeToCompanionSettingsChanged = (
  onChanged: (settings: CompanionSettings) => void,
) => listen<CompanionSettings>('companion-settings-changed', (event) => onChanged(event.payload));

export const subscribeToCompanionVisibilityChanged = (
  onChanged: (settings: CompanionSettings) => void,
) => listen<CompanionSettings>('companion-visibility-changed', (event) => onChanged(event.payload));
