import type { CSSProperties } from 'react';
import { companionAssetUrl, type CompanionSkin } from '../../lib/companion';

export type CompanionStatus = 'idle' | 'working' | 'success' | 'error';

const presetClass: Record<CompanionSkin['visualPreset'], string> = {
  quiet_aurora: 'companion-orb--aurora',
  porcelain_pearl: 'companion-orb--pearl',
  deep_ink: 'companion-orb--ink',
  custom: 'companion-orb--custom',
};

type CompanionOrbProps = {
  skin: CompanionSkin;
  motionEnabled: boolean;
  status: CompanionStatus;
};

export function CompanionOrb({ skin, motionEnabled, status }: CompanionOrbProps) {
  const style = {
    '--flow-primary': skin.flowColors[0] ?? '#BD9FFF',
    '--flow-secondary': skin.flowColors[1] ?? '#FFF4DC',
    '--flow-speed': skin.flowSpeed,
    '--flow-intensity': skin.flowIntensity,
  } as CSSProperties;

  return (
    <span
      className={[
        'companion-orb',
        presetClass[skin.visualPreset],
        `companion-orb--${status}`,
        motionEnabled ? '' : 'companion-orb--motion-off',
      ].filter(Boolean).join(' ')}
      style={style}
      data-testid="companion-orb"
      aria-hidden="true"
    >
      {skin.texturePath && <img className="companion-orb__texture" src={companionAssetUrl(skin.texturePath)} alt="" />}
      <span className="companion-orb__surface-current" />
      <span className="companion-orb__surface-sheen" />
      <span className="companion-orb__ring-track">
        <span className="companion-orb__ring-runner companion-orb__ring-runner--outer" />
        <span className="companion-orb__ring-runner companion-orb__ring-runner--inner" />
      </span>
      <span className="companion-orb__stars">
        {Array.from({ length: 5 }, (_, index) => (
          <span key={index} className="companion-orb__star-point" />
        ))}
      </span>
      <span className="companion-orb__central-star">✦</span>
    </span>
  );
}
