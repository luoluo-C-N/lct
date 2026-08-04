import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type Asset = {
  id: string;
  createdAt: string;
  importedAt: string;
  source: 'import' | 'capture';
  originalPath: string;
  previewPath: string;
  displayName: string;
  albumId: string | null;
  tags: string[];
  favorite: boolean;
  deletedAt: string | null;
  captureMode: 'fullscreen' | 'region' | 'window' | null;
  annotationData: string | null;
  syncVersion: number;
  cloudId: string | null;
};

export const listAssetsByMonth = (year: number, month: number) =>
  invoke<Asset[]>('list_assets_by_month', { year, month });

export const listAssetsByDay = (year: number, month: number, day: number) =>
  invoke<Asset[]>('list_assets_by_day', { year, month, day });

export const assetPreviewUrl = (path: string) => convertFileSrc(path);

export const subscribeToAssetCreated = (onAssetCreated: (asset: Asset) => void) =>
  listen<Asset>('asset-created', (event) => onAssetCreated(event.payload));
