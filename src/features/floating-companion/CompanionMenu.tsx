import type { CompanionSkin } from '../../lib/companion';

const skinNames: Record<string, string> = {
  'quiet-aurora': '静谧极光',
  'porcelain-pearl': '雾白珍珠',
  'deep-ink': '深海墨色',
};

type CompanionMenuProps = {
  skins: CompanionSkin[];
  activeSkinId: string;
  disabled: boolean;
  onSelectSkin: (skinId: string) => void;
  onCapture: (mode: 'region' | 'window' | 'fullscreen') => void;
  onImport: () => void;
  onOpenMain: () => void;
  onHide: () => void;
};

export function CompanionMenu({
  skins,
  activeSkinId,
  disabled,
  onSelectSkin,
  onCapture,
  onImport,
  onOpenMain,
  onHide,
}: CompanionMenuProps) {
  return (
    <menu className="companion-menu" aria-label="悬浮助手菜单">
      <li className="companion-menu-skins">
        <span>快速换肤</span>
        <div role="group" aria-label="快速切换皮肤">
          {skins.map((skin) => {
            const name = skinNames[skin.id] ?? skin.name;
            return (
              <button
                key={skin.id}
                type="button"
                aria-label={`切换到${name}`}
                aria-pressed={skin.id === activeSkinId}
                disabled={disabled || skin.id === activeSkinId}
                onClick={() => onSelectSkin(skin.id)}
              >
                {name}
              </button>
            );
          })}
        </div>
      </li>
      <li className="companion-menu-actions">
        <button type="button" disabled={disabled} onClick={() => onCapture('region')}>区域截图</button>
        <button type="button" disabled={disabled} onClick={() => onCapture('window')}>窗口截图</button>
        <button type="button" disabled={disabled} onClick={() => onCapture('fullscreen')}>全屏截图</button>
        <button type="button" disabled={disabled} onClick={onImport}>导入图片</button>
        <button type="button" disabled={disabled} onClick={onOpenMain}>打开影像库</button>
        <button type="button" disabled={disabled} onClick={onHide}>隐藏悬浮助手</button>
      </li>
    </menu>
  );
}
