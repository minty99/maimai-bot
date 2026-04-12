import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';

import { useI18n } from '../app/i18n';
import { daysSince, parseMaimaiPlayedAtToUnix } from '../app/maimaiTime';
import type {
  ScoreApiResponse,
  SongInfoResponse,
} from '../types';

interface ScatterPlotPageProps {
  sidebarTopContent?: ReactNode;
  scoreRecords: ScoreApiResponse[];
  songMetadata: Map<string, SongInfoResponse>;
}

interface PlotPoint {
  achievement: number;
  levelTenths: number;
  title: string;
}

const RANK_THRESHOLDS: Array<{ value: number; label: string; icon: string }> = [
  { value: 97.0, label: 'S', icon: '/rank-icons/s.png' },
  { value: 98.0, label: 'S+', icon: '/rank-icons/sp.png' },
  { value: 99.0, label: 'SS', icon: '/rank-icons/ss.png' },
  { value: 99.5, label: 'SS+', icon: '/rank-icons/ssp.png' },
  { value: 100.0, label: 'SSS', icon: '/rank-icons/sss.png' },
  { value: 100.5, label: 'SSS+', icon: '/rank-icons/sssp.png' },
];

const PALETTE = [
  '#5b9ef5',
  '#f0a050',
  '#6dd58c',
  '#e86080',
  '#a07af0',
  '#50c8c8',
  '#f07878',
  '#88b0e0',
  '#d0a060',
  '#c888e0',
];

interface PlotTheme {
  bg: string;
  paperBg: string;
  text: string;
  textMuted: string;
  titleColor: string;
  grid: string;
  laneSep: string;
  rankLine: string;
  rankLabel: string;
  rankLabelBg: string;
  hoverBg: string;
  hoverBorder: string;
  markerOutline: string;
}

const DARK_PLOT_THEME: PlotTheme = {
  bg: '#16161f',
  paperBg: '#0f0f17',
  text: '#c8c8d0',
  textMuted: '#8888a0',
  titleColor: '#e0e0e8',
  grid: 'rgba(255,255,255,0.06)',
  laneSep: 'rgba(255,255,255,0.10)',
  rankLine: 'rgba(255,255,255,0.18)',
  rankLabel: '#b0b0c0',
  rankLabelBg: 'rgba(22,22,31,0.85)',
  hoverBg: '#1e1e2e',
  hoverBorder: 'rgba(255,255,255,0.15)',
  markerOutline: 'rgba(0,0,0,0.35)',
};

const LIGHT_PLOT_THEME: PlotTheme = {
  bg: '#fafaf8',
  paperBg: '#f0f0ec',
  text: '#2a2a36',
  textMuted: '#6a6a88',
  titleColor: '#18181e',
  grid: 'rgba(0,0,0,0.06)',
  laneSep: 'rgba(0,0,0,0.10)',
  rankLine: 'rgba(0,0,0,0.22)',
  rankLabel: '#4a4a58',
  rankLabelBg: 'rgba(250,250,248,0.9)',
  hoverBg: '#ffffff',
  hoverBorder: 'rgba(0,0,0,0.15)',
  markerOutline: 'rgba(255,255,255,0.65)',
};

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

    // Watch data-theme attribute on <html>
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

    // Watch system preference
    const media = window.matchMedia('(prefers-color-scheme: light)');
    media.addEventListener('change', update);

    return () => {
      observer.disconnect();
      media.removeEventListener('change', update);
    };
  }, []);

  return theme;
}

const MIN_ACHIEVEMENT_FILTER = 90;
const DAYS_FILTER = 90;

function mulberry32(seed: number): () => number {
  let s = seed | 0;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function ScatterPlotPage({
  sidebarTopContent,
  scoreRecords,
  songMetadata,
}: ScatterPlotPageProps) {
  const { t } = useI18n();
  const plotRef = useRef<HTMLDivElement>(null);
  const effectiveTheme = useEffectiveTheme();
  const plotTheme = effectiveTheme === 'light' ? LIGHT_PLOT_THEME : DARK_PLOT_THEME;

  const [fromLevel, setFromLevel] = useState('13.0');
  const [toLevel, setToLevel] = useState('13.9');

  const handleFromChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setFromLevel(e.target.value);
  }, []);

  const handleToChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setToLevel(e.target.value);
  }, []);

  const fromTenths = Math.round(parseFloat(fromLevel || '0') * 10);
  const toTenths = Math.round(parseFloat(toLevel || '0') * 10);
  const isValidRange = Number.isFinite(fromTenths)
    && Number.isFinite(toTenths)
    && fromTenths >= 10
    && toTenths <= 150
    && fromTenths <= toTenths;

  const points = useMemo<PlotPoint[]>(() => {
    if (!isValidRange) return [];

    const levelMap = new Map<string, number>();
    for (const [, song] of songMetadata) {
      for (const sheet of song.sheets) {
        if (sheet.internal_level != null) {
          const ilTenths = Math.round(sheet.internal_level * 10);
          const key = JSON.stringify([song.title, song.genre, song.artist, sheet.chart_type, sheet.difficulty]);
          levelMap.set(key, ilTenths);
        }
      }
    }

    const result: PlotPoint[] = [];
    for (const score of scoreRecords) {
      if (score.achievement_x10000 == null) continue;
      const achievementPercent = score.achievement_x10000 / 10000;
      if (achievementPercent < MIN_ACHIEVEMENT_FILTER) continue;

      if (score.last_played_at) {
        const playedUnix = parseMaimaiPlayedAtToUnix(score.last_played_at);
        const elapsed = daysSince(playedUnix);
        if (elapsed != null && elapsed > DAYS_FILTER) continue;
      } else {
        continue;
      }

      const key = JSON.stringify([score.title, score.genre, score.artist, score.chart_type, score.diff_category]);
      const ilTenths = levelMap.get(key);
      if (ilTenths == null || ilTenths < fromTenths || ilTenths > toTenths) continue;

      result.push({
        achievement: achievementPercent,
        levelTenths: ilTenths,
        title: score.title,
      });
    }

    return result;
  }, [scoreRecords, songMetadata, fromTenths, toTenths, isValidRange]);

  const levels = useMemo(
    () => [...new Set(points.map((p) => p.levelTenths))].sort((a, b) => a - b),
    [points],
  );

  useEffect(() => {
    const el = plotRef.current;
    if (!el || points.length === 0 || levels.length === 0) {
      return;
    }

    let cancelled = false;

    void (async () => {
    const Plotly = (await import('plotly.js-dist-min')).default;
    if (cancelled) return;

    const rng = mulberry32(42);
    const nLevels = levels.length;
    const levelIndexMap = new Map(levels.map((lt, i) => [lt, i]));
    const colorMap = new Map(levels.map((lt, i) => [lt, PALETTE[i % PALETTE.length]]));
    const JITTER = 0.35;

    const traces = levels.map((levelTenths) => {
      const group = points.filter((p) => p.levelTenths === levelTenths);
      const idx = levelIndexMap.get(levelTenths) ?? 0;

      const xVals = group.map(() => idx + (rng() * 2 - 1) * JITTER);
      const yVals = group.map((p) => p.achievement);
      const hoverTexts = group.map(
        (p) => `<b>${p.title}</b><br>Lv ${(p.levelTenths / 10).toFixed(1)}<br>${p.achievement.toFixed(4)}%`,
      );

      return {
        x: xVals,
        y: yVals,
        mode: 'markers' as const,
        type: 'scatter' as const,
        name: `Lv ${(levelTenths / 10).toFixed(1)}`,
        text: hoverTexts,
        hoverinfo: 'text' as const,
        marker: {
          size: 11,
          color: colorMap.get(levelTenths),
          opacity: 0.85,
          line: { width: 0.6, color: plotTheme.markerOutline },
        },
      };
    });

    const minAchievement = Math.min(...points.map((p) => p.achievement));
    const yMin = Math.min(minAchievement, 100.5);

    const shapes: Array<Record<string, unknown>> = [];

    // Lane separators
    for (let i = 1; i < nLevels; i++) {
      shapes.push({
        type: 'line',
        x0: i - 0.5,
        x1: i - 0.5,
        y0: yMin,
        y1: 101.0,
        xref: 'x',
        yref: 'y',
        line: { dash: 'dash', color: plotTheme.laneSep, width: 1 },
      });
    }

    // Rank threshold lines
    const annotations: Array<Record<string, unknown>> = [];
    const images: Array<Record<string, unknown>> = [];
    const yRange = 101.0 - yMin;
    for (const rank of RANK_THRESHOLDS) {
      if (rank.value < yMin || rank.value > 101.0) continue;
      shapes.push({
        type: 'line',
        x0: -0.5,
        x1: nLevels - 0.5,
        y0: rank.value,
        y1: rank.value,
        xref: 'x',
        yref: 'y',
        line: { dash: 'dot', color: plotTheme.rankLine, width: 1.2 },
      });
      const paperY = (rank.value - yMin) / yRange;
      images.push({
        source: rank.icon,
        xref: 'paper',
        yref: 'paper',
        x: 1.01,
        y: paperY,
        sizex: 0.08,
        sizey: 0.04,
        xanchor: 'left',
        yanchor: 'middle',
        sizing: 'contain',
        layer: 'above',
      });
    }

    // Title
    const levelLabel = levels.length === 1
      ? `Lv ${(levels[0] / 10).toFixed(1)}`
      : `Lv ${(levels[0] / 10).toFixed(1)}\u2013${(levels[levels.length - 1] / 10).toFixed(1)}`;
    const titleText = `${levelLabel}  \u2014  ${points.length} song${points.length !== 1 ? 's' : ''} (last 3 months, \u226590%)`;

    const figWidth = Math.min(1200, Math.max(450, 110 * nLevels + 220));

    const layout: Record<string, unknown> = {
      title: {
        text: titleText,
        font: { size: 16, color: plotTheme.titleColor },
        x: 0.02,
        xanchor: 'left',
        y: 0.97,
        yanchor: 'top',
      },
      xaxis: {
        range: [-0.5, nLevels - 0.5],
        tickvals: levels.map((_, i) => i),
        ticktext: levels.map((lt) => `${(lt / 10).toFixed(1)}`),
        showgrid: false,
        zeroline: false,
        fixedrange: true,
        title: { text: 'Internal Level', font: { size: 12, color: plotTheme.textMuted } },
        tickfont: { size: 11, color: plotTheme.text },
      },
      yaxis: {
        range: [yMin, 101.0],
        title: { text: 'Achievement %', font: { size: 12, color: plotTheme.textMuted } },
        tickformat: '.2f',
        showgrid: true,
        gridcolor: plotTheme.grid,
        zeroline: false,
        fixedrange: true,
        tickfont: { size: 11, color: plotTheme.text },
      },
      plot_bgcolor: plotTheme.bg,
      paper_bgcolor: plotTheme.paperBg,
      showlegend: false,
      margin: { l: 70, r: 110, t: 60, b: 55 },
      shapes,
      annotations,
      images,
      width: figWidth,
      height: 650,
      hoverlabel: {
        bgcolor: plotTheme.hoverBg,
        bordercolor: plotTheme.hoverBorder,
        font: { color: plotTheme.text, size: 12 },
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
      void import('plotly.js-dist-min').then(({ default: P }) => P.purge(el));
    };
  }, [points, levels, plotTheme]);

  return (
    <div className="scatter-plot-page">
      {sidebarTopContent}
      <section className="scatter-plot-section panel">
        <h2 className="section-heading">{t('plot.title')}</h2>
        <p className="muted scatter-plot-description">{t('plot.description')}</p>

        <div className="scatter-plot-controls">
          <label className="scatter-plot-label">
            <span className="scatter-plot-label-text">From</span>
            <input
              type="number"
              className="scatter-plot-input"
              value={fromLevel}
              onChange={handleFromChange}
              min="1.0"
              max="15.0"
              step="0.1"
            />
          </label>
          <label className="scatter-plot-label">
            <span className="scatter-plot-label-text">To</span>
            <input
              type="number"
              className="scatter-plot-input"
              value={toLevel}
              onChange={handleToChange}
              min="1.0"
              max="15.0"
              step="0.1"
            />
          </label>
          {isValidRange && points.length > 0 ? (
            <span className="scatter-plot-summary muted">
              {levels.length === 1
                ? `Lv ${(levels[0] / 10).toFixed(1)}`
                : `Lv ${(levels[0] / 10).toFixed(1)}\u2013${(levels[levels.length - 1] / 10).toFixed(1)}`
              } &mdash; {points.length} song{points.length !== 1 ? 's' : ''}
            </span>
          ) : null}
        </div>

        {!isValidRange ? (
          <p className="muted scatter-plot-empty">{t('plot.invalidRange')}</p>
        ) : points.length === 0 ? (
          <p className="muted scatter-plot-empty">{t('plot.empty')}</p>
        ) : (
          <div className="scatter-plot-chart-container" ref={plotRef} />
        )}
      </section>
    </div>
  );
}
