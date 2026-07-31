import { useEffect, useState } from 'react';
import { MagicBookView } from './features/library/MagicBookView';
import { ClassicGallery } from './features/library/ClassicGallery';
import { SkinLibrary } from './features/skins/SkinLibrary';
import {
  getCompanionSettings,
  hideCompanion,
  showCompanion,
  subscribeToCompanionVisibilityChanged,
} from './lib/companion';
import './app.css';

type AppProps = {
  now?: () => Date;
};

export default function App({ now = () => new Date() }: AppProps) {
  const [view, setView] = useState<'book' | 'gallery' | 'skins'>('book');
  const [initialMonth] = useState(() => {
    const date = now();
    return { year: date.getFullYear(), month: date.getMonth() + 1 };
  });
  const [companionVisible, setCompanionVisible] = useState<boolean | null>(null);
  const [companionBusy, setCompanionBusy] = useState(false);
  const [companionError, setCompanionError] = useState('');

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void loadCompanionSettings()
      .then((settings) => {
        if (active) setCompanionVisible(settings.visible);
      })
      .catch(() => {
        if (active) setCompanionError('无法读取悬浮助手状态。');
      });
    void subscribeToCompanionVisibilityChanged((settings) => {
      if (active) setCompanionVisible(settings.visible);
    }).then((nextUnlisten) => {
      if (active) unlisten = nextUnlisten;
      else nextUnlisten();
    }).catch(() => {
      if (active) setCompanionError('无法监听悬浮助手状态。');
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  async function toggleCompanionVisibility() {
    if (companionVisible === null || companionBusy) return;
    setCompanionBusy(true);
    setCompanionError('');
    try {
      const settings = companionVisible ? await hideCompanion() : await showCompanion();
      setCompanionVisible(settings.visible);
    } catch {
      setCompanionError('悬浮助手状态更新失败，请重试。');
    } finally {
      setCompanionBusy(false);
    }
  }

  return (
    <main aria-label="影像资料库" className="application-shell">
      <header className="application-header">
        <strong>魔法影像库</strong>
        <div className="application-actions">
          <div role="group" aria-label="浏览模式">
            <button className={view === 'book' ? 'selected' : ''} onClick={() => setView('book')}>魔法书</button>
            <button className={view === 'gallery' ? 'selected' : ''} onClick={() => setView('gallery')}>图库</button>
            <button className={view === 'skins' ? 'selected' : ''} onClick={() => setView('skins')}>皮肤库</button>
          </div>
          <button
            type="button"
            className="companion-visibility-button"
            disabled={companionVisible === null || companionBusy}
            onClick={() => void toggleCompanionVisibility()}
          >
            {companionVisible === null
              ? '悬浮助手状态加载中'
              : companionVisible ? '隐藏悬浮助手' : '显示悬浮助手'}
          </button>
        </div>
      </header>
      {companionError && <p className="application-alert" role="alert">{companionError}</p>}
      {view === 'book' && <MagicBookView initialMonth={initialMonth} />}
      {view === 'gallery' && <ClassicGallery initialMonth={initialMonth} />}
      {view === 'skins' && <SkinLibrary />}
    </main>
  );
}

async function loadCompanionSettings() {
  try {
    return await getCompanionSettings();
  } catch {
    await new Promise((resolve) => setTimeout(resolve, 100));
    return getCompanionSettings();
  }
}
