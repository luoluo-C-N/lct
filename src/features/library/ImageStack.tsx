import { useEffect, useState, type CSSProperties } from 'react';
import { assetPreviewUrl, type Asset } from '../../lib/assets';

export function ImageStack({ assets }: { assets: Asset[] }) {
  const [imageIndex, setImageIndex] = useState(0);

  useEffect(() => {
    setImageIndex(0);
  }, [assets]);

  return (
    <>
      {assets.map((asset, index) => (
        <article
          className="memory-card"
          style={{
            '--offset': index - imageIndex,
            ...(assets.length === 1 ? { left: '35%' } : {}),
          } as CSSProperties}
          key={asset.id}
        >
          <img
            src={assetPreviewUrl(asset.previewPath)}
            alt={asset.id}
            style={{ width: '100%', height: '100%', objectFit: 'cover' }}
          />
        </article>
      ))}
      {assets.length > 1 && (
        <input
          aria-label="图片浏览滑轨"
          className="image-track"
          type="range"
          min="0"
          max={assets.length - 1}
          value={imageIndex}
          onChange={(event) => setImageIndex(Number(event.target.value))}
        />
      )}
    </>
  );
}
