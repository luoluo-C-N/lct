import { useEffect, useState } from 'react';
import type { CropRegion } from '../../lib/desktop';

type Point = { x: number; y: number };

type RegionOverlayProps = {
  scaleFactor: number;
  previewDataUrl: string;
  error?: string;
  disabled?: boolean;
  onSelect: (region: CropRegion) => void;
  onCancel: () => void;
};

export function RegionOverlay({
  scaleFactor,
  previewDataUrl,
  error,
  disabled = false,
  onSelect,
  onCancel,
}: RegionOverlayProps) {
  const [start, setStart] = useState<Point | null>(null);
  const [current, setCurrent] = useState<Point | null>(null);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!disabled && event.key === 'Escape') {
        event.preventDefault();
        onCancel();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [disabled, onCancel]);

  const selection = start && current ? normalizeSelection(start, current) : null;

  function begin(event: React.PointerEvent<HTMLDivElement>) {
    if (disabled || event.button !== 0) return;
    const point = { x: event.clientX, y: event.clientY };
    setStart(point);
    setCurrent(point);
  }

  function move(event: React.PointerEvent<HTMLDivElement>) {
    if (disabled || !start || (event.buttons & 1) === 0) return;
    setCurrent({ x: event.clientX, y: event.clientY });
  }

  function finish(event: React.PointerEvent<HTMLDivElement>) {
    if (disabled || !start || event.button !== 0) return;
    const logical = normalizeSelection(start, { x: event.clientX, y: event.clientY });
    setStart(null);
    setCurrent(null);
    if (logical.width === 0 || logical.height === 0) return;
    onSelect({
      x: Math.round(logical.x * scaleFactor),
      y: Math.round(logical.y * scaleFactor),
      width: Math.round(logical.width * scaleFactor),
      height: Math.round(logical.height * scaleFactor),
    });
  }

  return (
    <div
      className="region-overlay"
      role="dialog"
      aria-label="选择截图区域"
      aria-busy={disabled}
      onPointerDown={begin}
      onPointerMove={move}
      onPointerUp={finish}
      onPointerCancel={() => {
        setStart(null);
        setCurrent(null);
      }}
    >
      <img
        className="region-overlay__preview"
        src={previewDataUrl}
        alt="截图冻结画面"
        draggable={false}
      />
      {error && <p className="region-overlay__alert" role="alert">{error}</p>}
      {selection && (
        <span
          className="region-overlay__selection"
          style={{
            left: selection.x,
            top: selection.y,
            width: selection.width,
            height: selection.height,
          }}
        />
      )}
    </div>
  );
}

function normalizeSelection(start: Point, end: Point) {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}
