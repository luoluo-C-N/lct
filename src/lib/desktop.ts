import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { Asset } from './assets';
import type { SkinImportSelection } from './companion';

export type CaptureMode = 'region' | 'window' | 'fullscreen';

export type CropRegion = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export function capture(mode: CaptureMode, region?: CropRegion) {
  return invoke('capture', region ? { mode, region } : { mode });
}

export function importFiles(paths: string[]) {
  return invoke<Asset[]>('import_files', { paths });
}

export async function selectImageFiles() {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [{
      name: '图片',
      extensions: ['png', 'jpg', 'jpeg'],
    }],
  });

  if (!selected) return [];
  return Array.isArray(selected) ? selected : [selected];
}

export async function selectCompanionSkinFile(): Promise<SkinImportSelection | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{
      name: '皮肤文件',
      extensions: ['png', 'webp', 'zip'],
    }],
  });
  const path = Array.isArray(selected) ? selected[0] : selected;
  if (!path) return null;
  return {
    path,
    kind: path.toLowerCase().endsWith('.zip') ? 'package' : 'image',
  };
}
