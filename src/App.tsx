import { useState } from 'react';
import { MagicBookView } from './features/library/MagicBookView';
import { ClassicGallery } from './features/library/ClassicGallery';
import { SkinLibrary } from './features/skins/SkinLibrary';
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

  return (
    <main aria-label="影像资料库" className="application-shell">
      <header className="application-header">
        <strong>魔法影像库</strong>
        <div role="group" aria-label="浏览模式">
          <button className={view === 'book' ? 'selected' : ''} onClick={() => setView('book')}>魔法书</button>
          <button className={view === 'gallery' ? 'selected' : ''} onClick={() => setView('gallery')}>图库</button>
          <button className={view === 'skins' ? 'selected' : ''} onClick={() => setView('skins')}>皮肤库</button>
        </div>
      </header>
      {view === 'book' && <MagicBookView initialMonth={initialMonth} />}
      {view === 'gallery' && <ClassicGallery initialMonth={initialMonth} />}
      {view === 'skins' && <SkinLibrary />}
    </main>
  );
}
