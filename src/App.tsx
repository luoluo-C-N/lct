import { useState } from 'react';
import { MagicBookView } from './features/library/MagicBookView';
import { ClassicGallery } from './features/library/ClassicGallery';
import { CompanionWindow } from './features/floating-companion/CompanionWindow';
import './app.css';

export default function App() {
  const [view, setView] = useState<'book' | 'gallery'>('book');
  return (
    <main aria-label="影像资料库" className="application-shell">
      <header className="application-header">
        <strong>魔法影像库</strong>
        <div role="group" aria-label="浏览模式">
          <button className={view === 'book' ? 'selected' : ''} onClick={() => setView('book')}>魔法书</button>
          <button className={view === 'gallery' ? 'selected' : ''} onClick={() => setView('gallery')}>图库</button>
        </div>
      </header>
      {view === 'book'
        ? <MagicBookView initialMonth={{ year: 2026, month: 7 }} />
        : <ClassicGallery initialMonth={{ year: 2026, month: 7 }} />}
      <CompanionWindow />
    </main>
  );
}
