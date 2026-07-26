import { convertFileSrc, invoke } from '@tauri-apps/api/core';

export type Asset = {
  id: string;
  createdAt: string;
  importedAt: string;
  source: 'import' | 'capture';
  originalPath: string;
  previewPath: string;
  albumId: string | null;
  favorite: boolean;
  syncVersion: number;
};

export const listAssetsByMonth = (year: number, month: number) =>
  invoke<Asset[]>('list_assets_by_month', { year, month });

export const listAssetsByDay = (year: number, month: number, day: number) =>
  invoke<Asset[]>('list_assets_by_day', { year, month, day });

export const assetPreviewUrl = (path: string) => convertFileSrc(path);
