import { useEffect, useState } from 'react';
import {
  assetPreviewUrl,
  listAssetsByMonth,
  subscribeToAssetCreated,
  type Asset,
} from '../../lib/assets';
import { MonthPicker, type Month } from './MonthPicker';

type LoadMonth = (year: number, month: number) => Promise<Asset[]>;
type RequestStatus = 'loading' | 'ready' | 'error';

type ClassicGalleryProps = {
  initialMonth: Month;
  loadMonth?: LoadMonth;
};

export function ClassicGallery({
  initialMonth,
  loadMonth = listAssetsByMonth,
}: ClassicGalleryProps) {
  const [month, setMonth] = useState(initialMonth);
  const [assets, setAssets] = useState<Asset[]>([]);
  const [status, setStatus] = useState<RequestStatus>('loading');
  const [retryVersion, setRetryVersion] = useState(0);

  useEffect(() => {
    let active = true;
    setStatus('loading');
    setAssets([]);

    async function load() {
      try {
        const nextAssets = await loadMonth(month.year, month.month);
        if (!active) return;
        setAssets(nextAssets);
        setStatus('ready');
      } catch {
        if (active) setStatus('error');
      }
    }

    void load();
    return () => {
      active = false;
    };
  }, [loadMonth, month.month, month.year, retryVersion]);

  useEffect(() => {
    let disposed = false;
    let stopListening: (() => void) | undefined;

    void subscribeToAssetCreated((asset) => {
      const [assetYear, assetMonth] = asset.createdAt.slice(0, 7).split('-').map(Number);
      if (assetYear === month.year && assetMonth === month.month) {
        setRetryVersion((version) => version + 1);
      }
    }).then((unlisten) => {
      if (disposed) unlisten();
      else stopListening = unlisten;
    });

    return () => {
      disposed = true;
      stopListening?.();
    };
  }, [month.month, month.year]);

  return (
    <section className="classic-gallery" aria-label="传统图库">
      <div className="classic-gallery-toolbar">
        <MonthPicker month={month} onSelect={setMonth} />
      </div>
      {status === 'loading' && <p role="status">正在加载本月图片…</p>}
      {status === 'error' && (
        <div className="classic-gallery-state" role="alert">
          <p>无法加载本月图片</p>
          <button type="button" onClick={() => setRetryVersion((version) => version + 1)}>
            重试
          </button>
        </div>
      )}
      {status === 'ready' && assets.length === 0 && (
        <p className="classic-gallery-state">这个月还没有影像</p>
      )}
      {status === 'ready' && assets.length > 0 && (
        <div className="classic-gallery-grid">
          {assets.map((asset) => (
            <img
              key={asset.id}
              src={assetPreviewUrl(asset.previewPath)}
              alt={asset.id}
            />
          ))}
        </div>
      )}
    </section>
  );
}
