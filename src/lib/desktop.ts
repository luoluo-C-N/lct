import { invoke } from '@tauri-apps/api/core';

export type CaptureMode = 'region' | 'window' | 'fullscreen';

export function capture(mode: CaptureMode) {
  return invoke('capture', { mode });
}

export function importFiles(paths: string[]) {
  return invoke('import_files', { paths });
}
