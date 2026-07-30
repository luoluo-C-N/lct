import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

export type Month = {
  year: number;
  month: number;
};

type MonthPickerProps = {
  month: Month;
  onSelect: (month: Month) => void;
};

const months = Array.from({ length: 12 }, (_, index) => index + 1);

export function MonthPicker({ month, onSelect }: MonthPickerProps) {
  const [open, setOpen] = useState(false);
  const [year, setYear] = useState(month.year);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const dialogId = useId();

  useEffect(() => {
    if (!open) return;
    const dialog = dialogRef.current;
    if (!dialog) return;

    dialog.showModal();
    dialog.querySelector<HTMLButtonElement>('button')?.focus();

    return () => {
      if (dialog.open) dialog.close();
    };
  }, [open]);

  function openShelf() {
    setYear(month.year);
    setOpen(true);
  }

  function closeShelf() {
    if (dialogRef.current?.open) dialogRef.current.close();
    setOpen(false);
    triggerRef.current?.focus();
  }

  function selectMonth(selectedMonth: number) {
    onSelect({ year, month: selectedMonth });
    closeShelf();
  }

  return (
    <div className="month-picker">
      <button
        ref={triggerRef}
        className="month-picker-trigger"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={dialogId}
        onClick={openShelf}
      >
        <span aria-hidden="true">✦</span>
        {month.year} 年 {month.month} 月
      </button>
      {open && createPortal(
        <dialog
          ref={dialogRef}
          id={dialogId}
          className="month-picker-shelf"
          aria-label="选择月份"
          onCancel={(event) => {
            event.preventDefault();
            closeShelf();
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault();
              closeShelf();
            }
          }}
        >
          <header>
            <button type="button" aria-label="上一年" onClick={() => setYear((value) => value - 1)}>
              ‹
            </button>
            <strong aria-live="polite">{year} 年月度影集</strong>
            <button type="button" aria-label="下一年" onClick={() => setYear((value) => value + 1)}>
              ›
            </button>
          </header>
          <div className="month-picker-volumes">
            {months.map((value) => (
              <button
                key={value}
                type="button"
                aria-label={`${value} 月`}
                aria-current={year === month.year && value === month.month ? 'date' : undefined}
                onClick={() => selectMonth(value)}
              >
                <span>{String(value).padStart(2, '0')}</span>
                {value} 月
              </button>
            ))}
          </div>
          <button className="month-picker-close" type="button" onClick={closeShelf}>
            合上书册
          </button>
        </dialog>,
        document.body,
      )}
    </div>
  );
}
