import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  beginCompanionRegionSelection,
  finishCompanionRegionSelection,
  focusMainWindow,
  getCompanionSettings,
  hideCompanion,
  listCompanionSkins,
  setActiveCompanionSkin,
  setCompanionExpanded,
  subscribeToCompanionSettingsChanged,
  subscribeToCompanionSkinChanged,
  type CompanionSettings,
  type CompanionSkin,
  type CompanionSkinState,
} from '../../lib/companion';
import {
  capture,
  importFiles,
  selectImageFiles,
  type CaptureMode,
  type CropRegion,
} from '../../lib/desktop';
import { CompanionMenu } from './CompanionMenu';
import { CompanionOrb, type CompanionStatus } from './CompanionOrb';
import { RegionOverlay } from './RegionOverlay';
import { useCompanionPosition, type CompanionWindowApi } from './useCompanionPosition';

const fallbackSkin: CompanionSkin = {
  id: 'quiet-aurora',
  name: 'Quiet Aurora',
  source: 'builtin',
  visualPreset: 'quiet_aurora',
  texturePath: null,
  previewPath: null,
  flowColors: ['#BD9FFF', '#FFF4DC'],
  flowSpeed: 1,
  flowIntensity: 0.7,
  createdAt: '',
};

type CompanionWindowProps = {
  windowApi?: CompanionWindowApi;
};

export function CompanionWindow({
  windowApi = getCurrentWindow() as unknown as CompanionWindowApi,
}: CompanionWindowProps) {
  const [open, setOpen] = useState(false);
  const [skins, setSkins] = useState<CompanionSkin[]>([fallbackSkin]);
  const [settings, setSettings] = useState<CompanionSettings>({
    activeSkinId: fallbackSkin.id,
    motionEnabled: true,
    visible: true,
    placement: null,
  });
  const [status, setStatus] = useState<CompanionStatus>('idle');
  const [lastAction, setLastAction] = useState('');
  const [error, setError] = useState('');
  const [horizontalAnchor, setHorizontalAnchor] = useState<'left' | 'right'>('right');
  const [verticalAnchor, setVerticalAnchor] = useState<'top' | 'bottom'>('bottom');
  const [regionScaleFactor, setRegionScaleFactor] = useState<number | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const dragOriginRef = useRef<{ x: number; y: number } | null>(null);
  const suppressClickRef = useRef(false);

  useCompanionPosition(windowApi, undefined, !open);

  const activeSkin = useMemo(
    () => skins.find((skin) => skin.id === settings.activeSkinId) ?? skins[0] ?? fallbackSkin,
    [settings.activeSkinId, skins],
  );

  useEffect(() => {
    let active = true;
    void Promise.all([listCompanionSkins(), getCompanionSettings()])
      .then(([nextSkins, nextSettings]) => {
        if (!active) return;
        setSkins(nextSkins);
        setSettings(nextSettings);
      })
      .catch(() => {
        if (active) setError('悬浮助手状态加载失败。');
      });
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const register = (subscription: Promise<() => void>) => {
      void subscription.then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      }).catch(() => {
        if (!disposed) setError('悬浮助手同步已中断。');
      });
    };
    register(subscribeToCompanionSkinChanged(applySkinState));
    register(subscribeToCompanionSettingsChanged(setSettings));
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (!open) return undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        void closeMenu();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open]);

  function applySkinState(state: CompanionSkinState) {
    setSkins(state.skins);
    setSettings((current) => ({ ...current, activeSkinId: state.activeSkinId }));
  }

  async function toggleMenu() {
    if (open) await closeMenu();
    else {
      setError('');
      try {
        await detectAnchor();
        await setCompanionExpanded(true);
        setOpen(true);
      } catch {
        setError('无法展开悬浮助手。');
      }
    }
  }

  async function detectAnchor() {
    if (!windowApi.outerPosition || !windowApi.currentMonitor) return;
    const [position, monitor] = await Promise.all([
      windowApi.outerPosition(),
      windowApi.currentMonitor(),
    ]);
    if (!monitor) return;
    const { workArea } = monitor;
    setHorizontalAnchor(
      position.x + 36 < workArea.position.x + workArea.size.width / 2 ? 'left' : 'right',
    );
    setVerticalAnchor(
      position.y + 36 < workArea.position.y + workArea.size.height / 2 ? 'top' : 'bottom',
    );
  }

  async function closeMenu() {
    setError('');
    try {
      await setCompanionExpanded(false);
      setOpen(false);
      triggerRef.current?.focus();
    } catch {
      setError('无法收起悬浮助手。');
    }
  }

  function beginDragCandidate(event: React.PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    dragOriginRef.current = { x: event.clientX, y: event.clientY };
    suppressClickRef.current = false;
  }

  function startDragging(event: React.PointerEvent<HTMLButtonElement>) {
    const origin = dragOriginRef.current;
    if (!origin || (event.buttons & 1) === 0) return;
    if (Math.hypot(event.clientX - origin.x, event.clientY - origin.y) < 6) return;
    dragOriginRef.current = null;
    suppressClickRef.current = true;
    void windowApi.startDragging().catch(() => {
      suppressClickRef.current = false;
      setError('无法拖动悬浮助手。');
    });
  }

  function finishDragCandidate() {
    dragOriginRef.current = null;
  }

  function clickTrigger() {
    if (suppressClickRef.current) {
      suppressClickRef.current = false;
      return;
    }
    void toggleMenu();
  }

  async function selectSkin(skinId: string) {
    setStatus('working');
    setError('');
    try {
      applySkinState(await setActiveCompanionSkin(skinId));
      setStatus('success');
      setLastAction('皮肤已切换');
    } catch {
      setStatus('error');
      setError('皮肤切换失败，请重试。');
    }
  }

  async function startCapture(mode: CaptureMode) {
    setError('');
    setStatus('working');
    setLastAction(`${mode} 截图中`);
    try {
      if (mode === 'region') {
        await setCompanionExpanded(false);
        setOpen(false);
        const session = await beginCompanionRegionSelection();
        setRegionScaleFactor(session.scaleFactor);
        setLastAction('拖拽选择截图区域');
        return;
      }
      await capture(mode);
      setStatus('success');
      setLastAction(`${mode} 截图已保存`);
    } catch {
      setStatus('error');
      setLastAction('');
      setError('截图失败，请重试。');
    }
  }

  async function completeRegionCapture(region: CropRegion) {
    try {
      await finishCompanionRegionSelection();
      setRegionScaleFactor(null);
      await capture('region', region);
      setStatus('success');
      setLastAction('region 截图已保存');
    } catch {
      setStatus('error');
      setLastAction('');
      setError('截图失败，请重试。');
    }
  }

  async function cancelRegionCapture() {
    try {
      await finishCompanionRegionSelection();
      setRegionScaleFactor(null);
      setStatus('idle');
      setLastAction('已取消区域截图');
    } catch {
      setError('无法恢复悬浮助手窗口，请重试。');
    }
  }

  async function startImport() {
    setError('');
    setStatus('working');
    try {
      const paths = await selectImageFiles();
      if (paths.length === 0) {
        setStatus('idle');
        setLastAction('未选择图片');
        return;
      }
      setLastAction('正在导入图片…');
      const assets = await importFiles(paths);
      setStatus('success');
      setLastAction(`已导入 ${assets.length} 张图片`);
    } catch {
      setStatus('error');
      setLastAction('');
      setError('导入失败，请重新选择图片。');
    }
  }

  async function openMain() {
    try {
      await focusMainWindow();
    } catch {
      setError('无法打开影像库。');
    }
  }

  async function hideWindow() {
    try {
      await hideCompanion();
    } catch {
      setError('无法隐藏悬浮助手。');
    }
  }

  if (regionScaleFactor !== null) {
    return (
      <RegionOverlay
        scaleFactor={regionScaleFactor}
        onSelect={(region) => void completeRegionCapture(region)}
        onCancel={() => void cancelRegionCapture()}
      />
    );
  }

  return (
    <aside
      className={`companion ${open ? 'companion--expanded' : ''} companion--anchor-${horizontalAnchor} companion--anchor-${verticalAnchor}`}
      aria-label="悬浮助手"
    >
      <button
        ref={triggerRef}
        type="button"
        className="companion-trigger"
        aria-label={open ? '收起悬浮助手菜单' : '打开悬浮助手菜单'}
        aria-haspopup="menu"
        aria-expanded={open}
        data-testid="companion-drag-handle"
        onPointerDown={beginDragCandidate}
        onPointerMove={startDragging}
        onPointerUp={finishDragCandidate}
        onPointerCancel={finishDragCandidate}
        onClick={clickTrigger}
      >
        <CompanionOrb skin={activeSkin} motionEnabled={settings.motionEnabled} status={status} />
      </button>
      {open && (
        <CompanionMenu
          skins={skins}
          activeSkinId={settings.activeSkinId}
          disabled={status === 'working'}
          onSelectSkin={(skinId) => void selectSkin(skinId)}
          onCapture={(mode) => void startCapture(mode)}
          onImport={() => void startImport()}
          onOpenMain={() => void openMain()}
          onHide={() => void hideWindow()}
        />
      )}
      {lastAction && <output aria-live="polite">{lastAction}</output>}
      {error && <p role="alert">{error}</p>}
    </aside>
  );
}
