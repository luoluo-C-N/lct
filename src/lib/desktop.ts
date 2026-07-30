import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { Asset } from './assets';

export type CaptureMode = 'region' | 'window' | 'fullscreen';

export function capture(mode: CaptureMode) {
  return invoke('capture', { mode });
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
