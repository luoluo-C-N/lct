import { useEffect, useState } from 'react';
import {
  deleteCompanionSkin,
  getCompanionSettings,
  importCompanionSkin,
  listCompanionSkins,
  setActiveCompanionSkin,
  subscribeToCompanionSettingsChanged,
  subscribeToCompanionSkinChanged,
  updateCompanionSkin,
  type CompanionSkin,
  type CompanionSkinState,
} from '../../lib/companion';
import { selectCompanionSkinFile } from '../../lib/desktop';
import { SkinCard } from './SkinCard';

type RequestState = 'loading' | 'ready' | 'failed';

export function SkinLibrary() {
  const [skins, setSkins] = useState<CompanionSkin[]>([]);
  const [activeSkinId, setActiveSkinId] = useState('');
  const [requestState, setRequestState] = useState<RequestState>('loading');
  const [busySkinId, setBusySkinId] = useState<string | null>(null);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    let active = true;
    void Promise.all([listCompanionSkins(), getCompanionSettings()])
      .then(([nextSkins, settings]) => {
        if (!active) return;
        setSkins(nextSkins);
        setActiveSkinId(settings.activeSkinId);
        setRequestState('ready');
      })
      .catch(() => {
        if (active) setRequestState('failed');
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
        if (!disposed) setError('无法监听皮肤变化，请重新打开皮肤库。');
      });
    };
    register(subscribeToCompanionSkinChanged(applySkinState));
    register(subscribeToCompanionSettingsChanged((settings) => {
      setActiveSkinId(settings.activeSkinId);
    }));
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  function applySkinState(state: CompanionSkinState) {
    setSkins(state.skins);
    setActiveSkinId(state.activeSkinId);
  }

  async function selectSkin(skinId: string) {
    setError('');
    setBusySkinId(skinId);
    try {
      applySkinState(await setActiveCompanionSkin(skinId));
    } catch {
      setError('皮肤切换失败，当前皮肤未更改。');
    } finally {
      setBusySkinId(null);
    }
  }

  async function importSkin() {
    setError('');
    setNotice('');
    try {
      const selection = await selectCompanionSkinFile();
      if (!selection) return;
      const imported = await importCompanionSkin(selection.path);
      setSkins((current) => [...current.filter((skin) => skin.id !== imported.id), imported]);
      setNotice(`已导入 ${imported.name}`);
    } catch {
      setError('皮肤导入失败，请检查文件后重试。');
    }
  }

  async function saveSkin(skin: CompanionSkin) {
    setError('');
    setBusySkinId(skin.id);
    try {
      applySkinState(await updateCompanionSkin(skin));
      setNotice(`已保存 ${skin.name}`);
    } catch {
      setError('皮肤设置保存失败，请重试。');
    } finally {
      setBusySkinId(null);
    }
  }

  async function deleteSkin(skin: CompanionSkin) {
    setError('');
    setBusySkinId(skin.id);
    try {
      if (skin.id === activeSkinId) {
        applySkinState(await setActiveCompanionSkin('quiet-aurora'));
      }
      applySkinState(await deleteCompanionSkin(skin.id));
      setNotice(`已删除 ${skin.name}`);
    } catch {
      setError('皮肤删除失败，皮肤仍保留在库中。');
    } finally {
      setBusySkinId(null);
    }
  }

  return (
    <section className="skin-library" aria-label="皮肤库">
      <header className="skin-library-toolbar">
        <div>
          <h2>悬浮助手皮肤</h2>
          <p>管理助手的外观与流光参数</p>
        </div>
        <button type="button" onClick={() => void importSkin()}>导入皮肤</button>
      </header>
      {error && <p className="skin-library-alert" role="alert">{error}</p>}
      {notice && <p className="skin-library-notice" aria-live="polite">{notice}</p>}
      {requestState === 'loading' && <p role="status">正在读取皮肤…</p>}
      {requestState === 'failed' && <p role="alert">皮肤库加载失败，请重新打开此视图。</p>}
      {requestState === 'ready' && (
        <div className="skin-grid">
          {skins.map((skin) => (
            <SkinCard
              key={skin.id}
              skin={skin}
              active={skin.id === activeSkinId}
              busy={busySkinId === skin.id}
              onSelect={(skinId) => void selectSkin(skinId)}
              onSave={(nextSkin) => void saveSkin(nextSkin)}
              onDelete={(nextSkin) => void deleteSkin(nextSkin)}
            />
          ))}
        </div>
      )}
    </section>
  );
}
