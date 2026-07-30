import { MonthPicker, type Month } from './MonthPicker';

type DateFlowProps = {
  month: Month;
  dates: string[];
  selectedDate: string | null;
  onSelectMonth: (month: Month) => void;
  onSelectDate: (date: string) => void;
};

const weekday = ['日', '一', '二', '三', '四', '五', '六'];

export function DateFlow({
  month,
  dates,
  selectedDate,
  onSelectMonth,
  onSelectDate,
}: DateFlowProps) {
  return (
    <aside aria-label="日期流">
      <MonthPicker month={month} onSelect={onSelectMonth} />
      <ul>
        {dates.map((date) => {
          const [year, selectedMonth, day] = date.split('-').map(Number);
          const dayLabel = `${String(selectedMonth).padStart(2, '0')} / ${String(day).padStart(2, '0')}　周${weekday[new Date(year, selectedMonth - 1, day).getDay()]}`;

          return (
            <li key={date}>
              <button
                type="button"
                aria-current={date === selectedDate ? 'date' : undefined}
                onClick={() => onSelectDate(date)}
              >
                {dayLabel}
              </button>
            </li>
          );
        })}
      </ul>
    </aside>
  );
}
