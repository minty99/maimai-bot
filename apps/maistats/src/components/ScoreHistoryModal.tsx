import { useEffect, useMemo, useRef, useState } from 'react';

import { useI18n } from '../app/i18n';
import { formatPercent } from '../app/utils';
import { ChartTypeLabel } from './ChartTypeLabel';
import { DifficultyLabel } from './DifficultyLabel';
import { Jacket } from './Jacket';
import type { ScoreHistoryPoint, ScoreRow } from '../types';

interface ScoreHistoryModalProps {
  selectedHistoryRow: ScoreRow | null;
  historyPoints: ScoreHistoryPoint[];
  isLoading: boolean;
  loadingErrorMessage: string | null;
  songInfoUrl: string;
  onClose: () => void;
}

interface HistoryPlotTheme {
  bg: string;
  paperBg: string;
  text: string;
  muted: string;
  grid: string;
  axis: string;
  accent: string;
  accentSoft: string;
  rankLine: string;
  hoverBg: string;
  hoverBorder: string;
  markerFill: string;
  markerLine: string;
}

const FONT_FAMILY = "'Pretendard Variable', Pretendard, -apple-system, BlinkMacSystemFont, 'Apple SD Gothic Neo', 'Malgun Gothic', sans-serif";
const RANK_THRESHOLDS: Array<{ value: number; label: string; icon: string }> = [
  { value: 97.0, label: 'S', icon: '/rank-icons/s.png' },
  { value: 98.0, label: 'S+', icon: '/rank-icons/sp.png' },
  { value: 99.0, label: 'SS', icon: '/rank-icons/ss.png' },
  { value: 99.5, label: 'SS+', icon: '/rank-icons/ssp.png' },
  { value: 100.0, label: 'SSS', icon: '/rank-icons/sss.png' },
  { value: 100.5, label: 'SSS+', icon: '/rank-icons/sssp.png' },
];

const DARK_HISTORY_THEME: HistoryPlotTheme = {
  bg: '#111116',
  paperBg: 'rgba(0,0,0,0)',
  text: '#dcdce8',
  muted: '#9898b0',
  grid: 'rgba(255,255,255,0.06)',
  axis: 'rgba(255,255,255,0.16)',
  accent: '#e09000',
  accentSoft: 'rgba(224,144,0,0.18)',
  rankLine: 'rgba(255,255,255,0.22)',
  hoverBg: '#181820',
  hoverBorder: 'rgba(224,144,0,0.35)',
  markerFill: '#f3c15a',
  markerLine: '#0b0b0f',
};

const LIGHT_HISTORY_THEME: HistoryPlotTheme = {
  bg: '#ffffff',
  paperBg: 'rgba(0,0,0,0)',
  text: '#252533',
  muted: '#6a6a78',
  grid: 'rgba(0,0,0,0.07)',
  axis: 'rgba(0,0,0,0.18)',
  accent: '#c07a00',
  accentSoft: 'rgba(192,122,0,0.16)',
  rankLine: 'rgba(0,0,0,0.24)',
  hoverBg: '#ffffff',
  hoverBorder: 'rgba(192,122,0,0.32)',
  markerFill: '#d58a0a',
  markerLine: '#ffffff',
};

function toUnixMillis(unixtime: number): number {
  return unixtime > 10_000_000_000 ? unixtime : unixtime * 1000;
}

function formatPointTime(unixtime: number, locale: string): string {
  return new Date(toUnixMillis(unixtime)).toLocaleString(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function resolveEffectiveTheme(): 'light' | 'dark' {
  const attr = document.documentElement.getAttribute('data-theme');
  if (attr === 'light') return 'light';
  if (attr === 'dark') return 'dark';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function useEffectiveTheme(): 'light' | 'dark' {
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    if (typeof window === 'undefined') return 'dark';
    return resolveEffectiveTheme();
  });

  useEffect(() => {
    const update = () => setTheme(resolveEffectiveTheme());
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

    const media = window.matchMedia('(prefers-color-scheme: light)');
    media.addEventListener('change', update);

    return () => {
      observer.disconnect();
      media.removeEventListener('change', update);
    };
  }, []);

  return theme;
}

function buildYRange(values: number[]): [number, number] {
  const minAchievement = Math.min(...values);
  const maxAchievement = Math.max(...values);
  const span = Math.max(maxAchievement - minAchievement, 0.08);
  const padding = Math.max(span * 0.18, 0.05);
  const nearbyRanks = RANK_THRESHOLDS
    .map((rank) => rank.value)
    .filter((value) => value >= minAchievement - padding * 1.6 && value <= maxAchievement + padding * 1.6);
  const yMin = Math.max(0, Math.min(minAchievement, ...nearbyRanks) - padding);
  const yMax = Math.min(101.2, Math.max(maxAchievement, ...nearbyRanks) + padding);
  return yMax > yMin ? [yMin, yMax] : [Math.max(0, yMin - 0.5), yMax + 0.5];
}

export function ScoreHistoryModal({
  selectedHistoryRow,
  historyPoints,
  isLoading,
  loadingErrorMessage,
  songInfoUrl,
  onClose,
}: ScoreHistoryModalProps) {
  const { locale, t } = useI18n();
  const plotRef = useRef<HTMLDivElement>(null);
  const effectiveTheme = useEffectiveTheme();
  const plotTheme = effectiveTheme === 'light' ? LIGHT_HISTORY_THEME : DARK_HISTORY_THEME;
  const shouldShowLoadingState = isLoading && historyPoints.length === 0;

  const sortedHistoryPoints = useMemo(
    () => [...historyPoints].sort((a, b) => a.playedAtUnix - b.playedAtUnix),
    [historyPoints],
  );

  useEffect(() => {
    const el = plotRef.current;
    if (!el || !selectedHistoryRow || sortedHistoryPoints.length === 0) {
      return;
    }

    let cancelled = false;

    void (async () => {
      const Plotly = (await import('plotly.js-dist-min')).default;
      if (cancelled) return;

      const xValues = sortedHistoryPoints.map((point) => new Date(toUnixMillis(point.playedAtUnix)));
      const yValues = sortedHistoryPoints.map((point) => point.achievementPercent);
      const [yMin, yMax] = buildYRange(yValues);
      const hoverTexts = sortedHistoryPoints.map((point) => [
        `<b>${formatPercent(point.achievementPercent)}</b>`,
        point.playedAtLabel ?? formatPointTime(point.playedAtUnix, locale),
      ].join('<br>'));

      const shapes: Array<Record<string, unknown>> = [];
      const images: Array<Record<string, unknown>> = [];
      const annotations: Array<Record<string, unknown>> = [];
      const ySpan = yMax - yMin;

      for (const rank of RANK_THRESHOLDS) {
        if (rank.value < yMin || rank.value > yMax) continue;
        shapes.push({
          type: 'line',
          x0: 0,
          x1: 1,
          y0: rank.value,
          y1: rank.value,
          xref: 'paper',
          yref: 'y',
          line: { dash: 'dot', color: plotTheme.rankLine, width: 1.2 },
          layer: 'below',
        });
        images.push({
          source: rank.icon,
          xref: 'paper',
          yref: 'paper',
          x: 1.01,
          y: (rank.value - yMin) / ySpan,
          sizex: 0.075,
          sizey: 0.045,
          xanchor: 'left',
          yanchor: 'middle',
          sizing: 'contain',
          layer: 'above',
        });
        annotations.push({
          xref: 'paper',
          yref: 'y',
          x: 1,
          y: rank.value,
          text: rank.label,
          showarrow: false,
          xanchor: 'right',
          yanchor: 'bottom',
          font: { size: 10, color: plotTheme.muted, family: FONT_FAMILY },
        });
      }

      const traces = [{
        x: xValues,
        y: yValues,
        type: 'scatter' as const,
        mode: 'lines+markers' as const,
        fill: sortedHistoryPoints.length > 1 ? 'tozeroy' : 'none',
        fillcolor: plotTheme.accentSoft,
        line: {
          color: plotTheme.accent,
          width: 3,
          shape: 'spline' as const,
          smoothing: 0.35,
        },
        marker: {
          size: 9,
          color: plotTheme.markerFill,
          line: { width: 1.5, color: plotTheme.markerLine },
        },
        text: hoverTexts,
        hoverinfo: 'text' as const,
        cliponaxis: false,
      }];

      const layout: Record<string, unknown> = {
        font: { family: FONT_FAMILY, color: plotTheme.text },
        xaxis: {
          title: { text: t('history.axisTime'), font: { size: 12, color: plotTheme.muted } },
          showgrid: true,
          gridcolor: plotTheme.grid,
          zeroline: false,
          fixedrange: true,
          tickfont: { size: 11, color: plotTheme.muted, family: FONT_FAMILY },
          linecolor: plotTheme.axis,
        },
        yaxis: {
          title: { text: t('history.axisAchievement'), font: { size: 12, color: plotTheme.muted } },
          range: [yMin, yMax],
          ticksuffix: '%',
          tickformat: '.2f',
          showgrid: true,
          gridcolor: plotTheme.grid,
          zeroline: false,
          fixedrange: true,
          tickfont: { size: 11, color: plotTheme.muted, family: FONT_FAMILY },
          linecolor: plotTheme.axis,
        },
        plot_bgcolor: plotTheme.bg,
        paper_bgcolor: plotTheme.paperBg,
        showlegend: false,
        margin: { l: 72, r: 104, t: 20, b: 56 },
        shapes,
        images,
        annotations,
        height: 390,
        autosize: true,
        hovermode: 'closest',
        hoverlabel: {
          bgcolor: plotTheme.hoverBg,
          bordercolor: plotTheme.hoverBorder,
          font: { color: plotTheme.text, size: 12, family: FONT_FAMILY },
        },
        dragmode: false,
      };

      const config: Record<string, unknown> = {
        displayModeBar: false,
        displaylogo: false,
        responsive: true,
        scrollZoom: false,
        doubleClick: false,
        staticPlot: false,
      };

      Plotly.react(el, traces, layout, config);
    })();

    return () => {
      cancelled = true;
      void import('plotly.js-dist-min').then(({ default: Plotly }) => Plotly.purge(el));
    };
  }, [locale, plotTheme, selectedHistoryRow, sortedHistoryPoints, t]);

  if (!selectedHistoryRow) {
    return null;
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section
        className="modal-card panel history-modal"
        onClick={(event) => event.stopPropagation()}
      >
        <h2>{t('history.title')}</h2>
        <div className="detail-content">
          <div className="detail-header">
            <div className="detail-song-summary">
              <Jacket
                songInfoUrl={songInfoUrl}
                imageName={selectedHistoryRow.imageName}
                title={selectedHistoryRow.title}
                className="detail-jacket"
              />
              <div className="history-summary-copy">
                <strong>{selectedHistoryRow.title}</strong>
                <div className="history-badges">
                  <ChartTypeLabel chartType={selectedHistoryRow.chartType} />
                  <DifficultyLabel difficulty={selectedHistoryRow.difficulty} className="difficulty-badge" />
                </div>
                <p className="muted">
                  {t('history.description')}
                </p>
              </div>
            </div>
            <button type="button" className="modal-close-button" onClick={onClose}>
              {t('common.close')}
            </button>
          </div>

          {shouldShowLoadingState ? (
            <p className="muted">{t('history.loading')}</p>
          ) : loadingErrorMessage ? (
            <p className="muted">{t('common.error')}: {loadingErrorMessage}</p>
          ) : historyPoints.length === 0 ? (
            <p className="muted">{t('history.empty')}</p>
          ) : (
            <div className="history-chart-panel">
              <div className="history-chart-stage">
                <div
                  ref={plotRef}
                  className="history-plotly-chart"
                  role="img"
                  aria-label={t('history.graphLabel', { title: selectedHistoryRow.title })}
                />
              </div>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
