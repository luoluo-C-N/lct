import { useEffect } from 'react';
import { saveCompanionPlacement, type WindowPlacement } from '../../lib/companion';

type MovedEvent = { payload: { x: number; y: number } };

export type CompanionWindowApi = {
  startDragging: () => Promise<void>;
  onMoved: (handler: (event: MovedEvent) => void) => Promise<() => void>;
  outerSize: () => Promise<{ width: number; height: number }>;
  scaleFactor: () => Promise<number>;
  outerPosition?: () => Promise<{ x: number; y: number }>;
  currentMonitor?: () => Promise<{
    workArea: {
      position: { x: number; y: number };
      size: { width: number; height: number };
    };
  } | null>;
};

export function useCompanionPosition(
  windowApi: CompanionWindowApi,
  persist: (placement: WindowPlacement) => Promise<unknown> = saveCompanionPlacement,
  enabled = true,
) {
  useEffect(() => {
    if (!enabled) return undefined;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;

    void windowApi.onMoved((event) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        void Promise.all([windowApi.outerSize(), windowApi.scaleFactor()])
          .then(([size, scaleFactor]) => {
            if (disposed) return;
            return persist({
              x: event.payload.x / scaleFactor,
              y: event.payload.y / scaleFactor,
              width: size.width / scaleFactor,
              height: size.height / scaleFactor,
            });
          })
          .catch(() => undefined);
      }, 250);
    }).then((stopListening) => {
      if (disposed) stopListening();
      else unlisten = stopListening;
    }).catch(() => undefined);

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      unlisten?.();
    };
  }, [enabled, persist, windowApi]);
}
