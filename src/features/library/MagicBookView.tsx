import { useEffect, useMemo, useRef, useState } from 'react';
import {
  listAssetsByDay,
  listAssetsByMonth,
  type Asset,
} from '../../lib/assets';
import { DateFlow } from './DateFlow';
import { ImageStack } from './ImageStack';

type Month = { year: number; month: number };
type LoadMonth = (year: number, month: number) => Promise<Asset[]>;
type LoadDay = (year: number, month: number, day: number) => Promise<Asset[]>;
type RequestState = 'loading' | 'ready' | 'failed';

type MagicBookViewProps = {
  initialMonth: Month;
  loadMonth?: LoadMonth;
  loadDay?: LoadDay;
};

const availableMonths = Array.from({ length: 12 }, (_, index) => index + 1);

export function MagicBookView({
  initialMonth,
  loadMonth = listAssetsByMonth,
  loadDay = listAssetsByDay,
}: MagicBookViewProps) {
  const [month, setMonth] = useState(initialMonth);
  const [monthAssets, setMonthAssets] = useState<Asset[]>([]);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [selectedDayAssets, setSelectedDayAssets] = useState<Asset[] | null>(null);
  const [requestState, setRequestState] = useState<RequestState>('loading');
  const [dayRequestState, setDayRequestState] = useState<RequestState>('ready');
  const [pickerOpen, setPickerOpen] = useState(false);
  const [retryVersion, setRetryVersion] = useState(0);
  const dayRequestVersion = useRef(0);

  const dates = useMemo(
    () => [...new Set(monthAssets.map((asset) => asset.createdAt.slice(0, 10)))].sort().reverse(),
    [monthAssets],
  );
  const selectedAssets = useMemo(
    () => selectedDayAssets ?? monthAssets.filter((asset) => asset.createdAt.slice(0, 10) === selectedDate),
    [monthAssets, selectedDate, selectedDayAssets],
  );

  useEffect(() => {
    let active = true;
    dayRequestVersion.current += 1;
    setRequestState('loading');
    setMonthAssets([]);
    setSelectedDate(null);
    setSelectedDayAssets(null);
    setDayRequestState('ready');

    async function requestMonth() {
      try {
        const assets = await loadMonth(month.year, month.month);
        if (!active) return;

        const nextDates = [...new Set(assets.map((asset) => asset.createdAt.slice(0, 10)))].sort().reverse();
        setMonthAssets(assets);
        setSelectedDate(nextDates[0] ?? null);
        setRequestState('ready');
      } catch {
        if (active) setRequestState('failed');
      }
    }

    void requestMonth();
    return () => {
      active = false;
    };
  }, [loadMonth, month, retryVersion]);

  function selectMonth(nextMonth: number) {
    dayRequestVersion.current += 1;
    setRequestState('loading');
    setMonthAssets([]);
    setSelectedDate(null);
    setSelectedDayAssets(null);
    setMonth({ year: month.year, month: nextMonth });
    setPickerOpen(false);
  }

  async function requestDay(date: string) {
    const requestVersion = ++dayRequestVersion.current;
    const [year, selectedMonth, day] = date.split('-').map(Number);
    setSelectedDayAssets(null);
    setDayRequestState('loading');

    try {
      const assets = await loadDay(year, selectedMonth, day);
      if (requestVersion !== dayRequestVersion.current) return;
      setSelectedDayAssets(assets);
      setDayRequestState('ready');
    } catch {
      if (requestVersion === dayRequestVersion.current) setDayRequestState('failed');
    }
  }

  function selectDate(date: string) {
    setSelectedDate(date);
    void requestDay(date);
  }

  function renderImageContent() {
    if (requestState === 'loading' || dayRequestState === 'loading') {
      return <p role="status">正在翻阅影像…</p>;
    }
    if (requestState === 'failed') {
      return (
        <div role="alert">
          <p>影像加载失败</p>
          <button type="button" onClick={() => setRetryVersion((version) => version + 1)}>重试</button>
        </div>
      );
    }
    if (dayRequestState === 'failed') {
      return (
        <div role="alert">
          <p>影像加载失败</p>
          <button type="button" onClick={() => selectedDate && void requestDay(selectedDate)}>重试</button>
        </div>
      );
    }
    if (monthAssets.length === 0) return <p>这个月还没有影像</p>;
    if (selectedAssets.length === 0) return <p>这一天还没有影像</p>;

    return <ImageStack assets={selectedAssets} />;
  }

  return (
    <main aria-label="魔法书资料库">
      <DateFlow
        month={month}
        dates={dates}
        selectedDate={selectedDate}
        onOpenMonthPicker={() => setPickerOpen(true)}
        onSelectDate={selectDate}
      />
      <section aria-label="图片叠页">{renderImageContent()}</section>
      {pickerOpen && (
        <dialog open aria-label="选择月份">
          {availableMonths.map((value) => (
            <button key={value} type="button" onClick={() => selectMonth(value)}>
              {value} 月
            </button>
          ))}
        </dialog>
      )}
    </main>
  );
}
