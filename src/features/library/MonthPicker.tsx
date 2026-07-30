import { useState } from 'react';

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

  function openShelf() {
    setYear(month.year);
    setOpen(true);
  }

  function selectMonth(selectedMonth: number) {
    onSelect({ year, month: selectedMonth });
    setOpen(false);
  }

  return (
    <div className="month-picker">
      <button className="month-picker-trigger" type="button" onClick={openShelf}>
        <span aria-hidden="true">✦</span>
        {month.year} 年 {month.month} 月
      </button>
      {open && (
        <dialog
          className="month-picker-shelf"
          open
          aria-label="选择月份"
          onCancel={() => setOpen(false)}
          onKeyDown={(event) => {
            if (event.key === 'Escape') setOpen(false);
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
          <button className="month-picker-close" type="button" onClick={() => setOpen(false)}>
            合上书册
          </button>
        </dialog>
      )}
    </div>
  );
}
