import { useEffect, useRef, useState } from 'react';
import type { CompanionSkin } from '../../lib/companion';

type SkinControlsProps = {
  skin: CompanionSkin;
  disabled: boolean;
  onSave: (skin: CompanionSkin) => void;
};

export function SkinControls({ skin, disabled, onSave }: SkinControlsProps) {
  const [name, setName] = useState(skin.name);
  const [primary, setPrimary] = useState(skin.flowColors[0] ?? '#BDA7FF');
  const [secondary, setSecondary] = useState(skin.flowColors[1] ?? '#55D8CF');
  const [speed, setSpeed] = useState(skin.flowSpeed);
  const [intensity, setIntensity] = useState(skin.flowIntensity);
  const persistedRevision = useRef(formRevision(skin));

  useEffect(() => {
    const nextRevision = formRevision(skin);
    if (nextRevision === persistedRevision.current) return;
    persistedRevision.current = nextRevision;
    setName(skin.name);
    setPrimary(skin.flowColors[0] ?? '#BDA7FF');
    setSecondary(skin.flowColors[1] ?? '#55D8CF');
    setSpeed(skin.flowSpeed);
    setIntensity(skin.flowIntensity);
  }, [skin]);

  const savedName = name.trim() || skin.name;

  return (
    <div className="skin-controls">
      <label>
        <span>皮肤名称</span>
        <input
          aria-label="皮肤名称"
          value={name}
          maxLength={48}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="skin-color-controls">
        <label>
          <span>主流光色</span>
          <input
            aria-label="主流光色"
            type="color"
            value={primary}
            onChange={(event) => setPrimary(event.target.value.toUpperCase())}
          />
        </label>
        <label>
          <span>辅流光色</span>
          <input
            aria-label="辅流光色"
            type="color"
            value={secondary}
            onChange={(event) => setSecondary(event.target.value.toUpperCase())}
          />
        </label>
      </div>
      <label>
        <span>流动速度 <output>{speed.toFixed(2)}</output></span>
        <input
          aria-label="流动速度"
          type="range"
          min="0.5"
          max="2"
          step="0.05"
          value={speed}
          onChange={(event) => setSpeed(Number(event.target.value))}
        />
      </label>
      <label>
        <span>流光强度 <output>{intensity.toFixed(2)}</output></span>
        <input
          aria-label="流光强度"
          type="range"
          min="0"
          max="1"
          step="0.05"
          value={intensity}
          onChange={(event) => setIntensity(Number(event.target.value))}
        />
      </label>
      <button
        type="button"
        className="skin-save-button"
        disabled={disabled}
        aria-label={`保存${savedName}`}
        onClick={() => onSave({
          ...skin,
          name: savedName,
          flowColors: [primary, secondary],
          flowSpeed: speed,
          flowIntensity: intensity,
        })}
      >
        保存更改
      </button>
    </div>
  );
}

function formRevision(skin: CompanionSkin) {
  return JSON.stringify([
    skin.id,
    skin.name,
    skin.flowColors,
    skin.flowSpeed,
    skin.flowIntensity,
  ]);
}
