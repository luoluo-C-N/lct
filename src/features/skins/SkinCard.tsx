import type { CompanionSkin } from '../../lib/companion';
import { SkinControls } from './SkinControls';
import { SkinPreview } from './SkinPreview';

const builtinNames: Record<string, string> = {
  'quiet-aurora': '静谧极光',
  'porcelain-pearl': '雾白珍珠',
  'deep-ink': '深海墨色',
};

type SkinCardProps = {
  skin: CompanionSkin;
  active: boolean;
  busy: boolean;
  onSelect: (skinId: string) => void;
  onSave: (skin: CompanionSkin) => void;
  onDelete: (skin: CompanionSkin) => void;
};

export const displaySkinName = (skin: CompanionSkin) => builtinNames[skin.id] ?? skin.name;

export function SkinCard({ skin, active, busy, onSelect, onSave, onDelete }: SkinCardProps) {
  const name = displaySkinName(skin);
  const local = skin.source !== 'builtin';

  return (
    <article className="skin-card" aria-label={name}>
      <SkinPreview skin={skin} />
      <div className="skin-card-heading">
        <div>
          <h3>{name}</h3>
          <span>{local ? (skin.source === 'package' ? '本地皮肤包' : '本地图片') : '内置皮肤'}</span>
        </div>
        {active && <strong>使用中</strong>}
      </div>
      <button
        type="button"
        className="skin-use-button"
        aria-label={`使用${name}`}
        aria-pressed={active}
        disabled={busy || active}
        onClick={() => onSelect(skin.id)}
      >
        {active ? '正在使用' : '使用此皮肤'}
      </button>
      {local && (
        <>
          <SkinControls skin={skin} disabled={busy} onSave={onSave} />
          <button
            type="button"
            className="skin-delete-button"
            aria-label={`删除${name}`}
            disabled={busy}
            onClick={() => onDelete(skin)}
          >
            删除皮肤
          </button>
        </>
      )}
    </article>
  );
}
