import type { CSSProperties } from 'react';
import { companionAssetUrl, type CompanionSkin, type VisualPreset } from '../../lib/companion';

const presetClass: Record<VisualPreset, string> = {
  quiet_aurora: 'skin-preview--aurora',
  porcelain_pearl: 'skin-preview--pearl',
  deep_ink: 'skin-preview--ink',
  custom: 'skin-preview--custom',
};

export function SkinPreview({ skin }: { skin: CompanionSkin }) {
  const style = {
    '--skin-primary': skin.flowColors[0] ?? '#BDA7FF',
    '--skin-secondary': skin.flowColors[1] ?? '#55D8CF',
  } as CSSProperties;

  return (
    <div className={`skin-preview ${presetClass[skin.visualPreset]}`} style={style} aria-hidden="true">
      {skin.previewPath && <img src={companionAssetUrl(skin.previewPath)} alt="" />}
      <span className="skin-preview-core" />
      <span className="skin-preview-ring" />
    </div>
  );
}
