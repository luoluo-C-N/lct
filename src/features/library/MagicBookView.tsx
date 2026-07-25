import { useMemo, useState } from 'react';

type Month = { year: number; month: number };

const datesByMonth: Record<number, number[]> = {
  5: [18, 19, 20, 21, 22],
  6: [16, 17, 18, 19, 20],
  7: [21, 22, 23, 24, 25, 26, 27, 28],
  8: [12, 13, 14, 15, 16],
};

const weekday = ['日', '一', '二', '三', '四', '五', '六'];

export function MagicBookView({ initialMonth }: { initialMonth: Month }) {
  const [month, setMonth] = useState(initialMonth);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [imageIndex, setImageIndex] = useState(0);
  const dates = useMemo(() => datesByMonth[month.month] ?? [], [month]);
  const [selectedDay, setSelectedDay] = useState(dates.at(-1) ?? 1);

  function selectMonth(nextMonth: number) {
    const nextDates = datesByMonth[nextMonth] ?? [];
    setMonth({ year: month.year, month: nextMonth });
    setSelectedDay(nextDates[Math.floor(nextDates.length / 2)] ?? 1);
    setPickerOpen(false);
  }

  return (
    <main aria-label="魔法书资料库">
      <aside aria-label="日期流">
        <button type="button" onClick={() => setPickerOpen(true)}>
          {month.year} 年 {month.month} 月
        </button>
        <ul>
          {dates.map((day) => (
            <li key={day}>
              <button
                type="button"
                aria-current={day === selectedDay ? 'date' : undefined}
                onClick={() => setSelectedDay(day)}
              >
                {`${String(month.month).padStart(2, '0')} / ${String(day).padStart(2, '0')}　周${weekday[new Date(month.year, month.month - 1, day).getDay()]}`}
              </button>
            </li>
          ))}
        </ul>
      </aside>
      <section aria-label="图片叠页">
        {['记住这一刻', '薄荷夏天', '午后光线', '云端备份'].map((title, index) => (
          <article className="memory-card" style={{ '--offset': index - imageIndex } as React.CSSProperties} key={title}>
            <strong>{title}</strong>
          </article>
        ))}
        <input aria-label="图片浏览滑轨" className="image-track" type="range" min="0" max="3" value={imageIndex} onChange={(event) => setImageIndex(Number(event.target.value))} />
      </section>
      {pickerOpen && (
        <dialog open aria-label="选择月份">
          {[5, 6, 7, 8].map((value) => (
            <button key={value} type="button" onClick={() => selectMonth(value)}>
              {value} 月
            </button>
          ))}
        </dialog>
      )}
    </main>
  );
}
