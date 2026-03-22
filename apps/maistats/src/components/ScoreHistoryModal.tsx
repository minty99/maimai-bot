import { useEffect, useState } from 'react';

import { useI18n } from '../app/i18n';
import { formatNumber, formatPercent } from '../app/utils';
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

const CHART_WIDTH = 820;
const CHART_HEIGHT = 360;
const CHART_MARGIN = { top: 18, right: 28, bottom: 68, left: 84 };
const POINT_RADIUS = 5;
const ACTIVE_POINT_RADIUS = 7;

interface ChartPoint extends ScoreHistoryPoint {
  chartX: number;
  chartY: number;
}

function toUnixMillis(unixtime: number): number {
  return unixtime > 10_000_000_000 ? unixtime : unixtime * 1000;
}

function buildLinearTicks(start: number, end: number, count: number): number[] {
  if (count <= 1 || start === end) {
    return [start];
  }

  const step = (end - start) / (count - 1);
  return Array.from({ length: count }, (_, index) => start + step * index);
}

function formatDateTick(unixtime: number, spanMs: number, locale: string): string {
  const date = new Date(toUnixMillis(unixtime));
  return date.toLocaleDateString(locale, {
    year: spanMs > 1000 * 60 * 60 * 24 * 365 ? '2-digit' : undefined,
    month: '2-digit',
    day: '2-digit',
  });
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

function formatDeltaPercent(value: number): string {
  const formatted = formatPercent(Math.abs(value));

  if (value > 0) {
    return `+${formatted}`;
  }
  if (value < 0) {
    return `-${formatted}`;
  }
  return formatted;
}

function buildAreaPath(points: ChartPoint[], baselineY: number): string {
  if (points.length === 0) {
    return '';
  }

  const [firstPoint, ...remainingPoints] = points;
  const lastPoint = points[points.length - 1];

  return [
    `M ${firstPoint.chartX} ${baselineY}`,
    `L ${firstPoint.chartX} ${firstPoint.chartY}`,
    ...remainingPoints.map((point) => `L ${point.chartX} ${point.chartY}`),
    `L ${lastPoint.chartX} ${baselineY}`,
    'Z',
  ].join(' ');
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
  const [hoveredPointKey, setHoveredPointKey] = useState<string | null>(null);
  const shouldShowLoadingState = isLoading && historyPoints.length === 0;

  useEffect(() => {
    setHoveredPointKey(historyPoints[historyPoints.length - 1]?.key ?? null);
  }, [historyPoints]);

  if (!selectedHistoryRow) {
    return null;
  }

  const innerWidth = CHART_WIDTH - CHART_MARGIN.left - CHART_MARGIN.right;
  const innerHeight = CHART_HEIGHT - CHART_MARGIN.top - CHART_MARGIN.bottom;

  const xValues = historyPoints.map((point) => point.playedAtUnix);
  const yValues = historyPoints.map((point) => point.achievementPercent);

  const minX = xValues.length > 0 ? Math.min(...xValues) : 0;
  const maxX = xValues.length > 0 ? Math.max(...xValues) : 0;
  const minAchievement = yValues.length > 0 ? Math.min(...yValues) : 0;
  const maxAchievement = yValues.length > 0 ? Math.max(...yValues) : 100;
  const yPadding = Math.max((maxAchievement - minAchievement) * 0.18, 0.04);
  const yMin = Math.max(0, minAchievement - yPadding);
  const yMax = maxAchievement + yPadding;
  const effectiveYMax = yMax > yMin ? yMax : yMin + 1;
  const xSpan = maxX - minX;
  const ySpan = effectiveYMax - yMin;

  const toChartX = (value: number) => {
    if (historyPoints.length <= 1 || xSpan === 0) {
      return CHART_MARGIN.left + innerWidth / 2;
    }
    return CHART_MARGIN.left + ((value - minX) / xSpan) * innerWidth;
  };

  const toChartY = (value: number) => {
    const rawY = CHART_MARGIN.top + innerHeight - ((value - yMin) / ySpan) * innerHeight;
    return Math.min(
      CHART_MARGIN.top + innerHeight - POINT_RADIUS,
      Math.max(CHART_MARGIN.top + POINT_RADIUS, rawY),
    );
  };

  const xTicks = historyPoints.length === 0
    ? []
    : buildLinearTicks(minX, maxX, historyPoints.length === 1 ? 1 : Math.min(5, historyPoints.length));
  const yTicks = historyPoints.length === 0 ? [] : buildLinearTicks(yMin, effectiveYMax, 5);
  const chartPoints = historyPoints.map((point) => ({
    ...point,
    chartX: toChartX(point.playedAtUnix),
    chartY: toChartY(point.achievementPercent),
  }));
  const firstPoint = chartPoints[0] ?? null;
  const latestPoint = chartPoints[chartPoints.length - 1] ?? null;
  const hoveredPoint = chartPoints.find((point) => point.key === hoveredPointKey) ?? null;
  const activePoint = hoveredPoint ?? latestPoint;
  const linePoints = chartPoints
    .map((point) => `${point.chartX},${point.chartY}`)
    .join(' ');
  const areaPath = buildAreaPath(chartPoints, CHART_MARGIN.top + innerHeight);
  const totalGain = firstPoint && latestPoint
    ? latestPoint.achievementPercent - firstPoint.achievementPercent
    : 0;

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
            <div
              className="history-chart-panel"
              onMouseLeave={() => setHoveredPointKey(null)}
            >
              <div className="history-chart-topline">
                <div className="history-metric-strip" aria-label={t('history.graphLabel', { title: selectedHistoryRow.title })}>
                  <div className="history-metric">
                    <span className="history-metric-label">{t('history.currentBest')}</span>
                    <strong>{formatPercent(latestPoint?.achievementPercent ?? null)}</strong>
                  </div>
                  <div className="history-metric">
                    <span className="history-metric-label">{t('history.totalGain')}</span>
                    <strong>{formatDeltaPercent(totalGain)}</strong>
                  </div>
                  <div className="history-metric">
                    <span className="history-metric-label">{t('history.improvements')}</span>
                    <strong>{formatNumber(chartPoints.length, locale)}</strong>
                  </div>
                </div>

                {activePoint ? (
                  <div className="history-inspector" aria-live="polite">
                    <span className="history-inspector-label">
                      {hoveredPoint ? t('history.focusPoint') : t('history.latestPoint')}
                    </span>
                    <strong>{formatPercent(activePoint.achievementPercent)}</strong>
                    <span>{activePoint.playedAtLabel ?? formatPointTime(activePoint.playedAtUnix, locale)}</span>
                  </div>
                ) : null}
              </div>

              <div className="history-chart-stage">
                <svg
                  viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
                  className="history-chart"
                  role="img"
                  aria-label={t('history.graphLabel', { title: selectedHistoryRow.title })}
                >
                  <defs>
                    <linearGradient id="history-line-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
                      <stop className="history-line-stop-start" offset="0%" />
                      <stop className="history-line-stop-end" offset="100%" />
                    </linearGradient>
                    <linearGradient id="history-area-gradient" x1="0%" y1="0%" x2="0%" y2="100%">
                      <stop className="history-area-stop-start" offset="0%" />
                      <stop className="history-area-stop-end" offset="100%" />
                    </linearGradient>
                  </defs>

                  {yTicks.map((tick) => (
                    <g key={`y-${tick}`} className="history-grid">
                      <line
                        x1={CHART_MARGIN.left}
                        x2={CHART_MARGIN.left + innerWidth}
                        y1={toChartY(tick)}
                        y2={toChartY(tick)}
                        vectorEffect="non-scaling-stroke"
                      />
                      <text
                        x={CHART_MARGIN.left - 10}
                        y={toChartY(tick)}
                        textAnchor="end"
                        dominantBaseline="middle"
                      >
                        {formatPercent(tick)}
                      </text>
                    </g>
                  ))}

                  {xTicks.map((tick) => (
                    <g key={`x-${tick}`} className="history-grid history-grid-x">
                      <line
                        x1={toChartX(tick)}
                        x2={toChartX(tick)}
                        y1={CHART_MARGIN.top}
                        y2={CHART_MARGIN.top + innerHeight}
                        vectorEffect="non-scaling-stroke"
                      />
                      <text
                        x={toChartX(tick)}
                        y={CHART_MARGIN.top + innerHeight + 24}
                        textAnchor="middle"
                      >
                        {formatDateTick(tick, xSpan, locale)}
                      </text>
                    </g>
                  ))}

                  <line
                    className="history-axis"
                    x1={CHART_MARGIN.left}
                    x2={CHART_MARGIN.left}
                    y1={CHART_MARGIN.top}
                    y2={CHART_MARGIN.top + innerHeight}
                    vectorEffect="non-scaling-stroke"
                  />
                  <line
                    className="history-axis"
                    x1={CHART_MARGIN.left}
                    x2={CHART_MARGIN.left + innerWidth}
                    y1={CHART_MARGIN.top + innerHeight}
                    y2={CHART_MARGIN.top + innerHeight}
                    vectorEffect="non-scaling-stroke"
                  />

                  <text
                    className="history-axis-label"
                    x={24}
                    y={CHART_MARGIN.top + innerHeight / 2}
                    transform={`rotate(-90 24 ${CHART_MARGIN.top + innerHeight / 2})`}
                    textAnchor="middle"
                  >
                    {t('history.axisAchievement')}
                  </text>
                  <text
                    className="history-axis-label"
                    x={CHART_MARGIN.left + innerWidth / 2}
                    y={CHART_HEIGHT - 18}
                    textAnchor="middle"
                  >
                    {t('history.axisTime')}
                  </text>

                  {areaPath ? <path className="history-area" d={areaPath} /> : null}

                  {activePoint ? (
                    <line
                      className="history-active-guide"
                      x1={activePoint.chartX}
                      x2={activePoint.chartX}
                      y1={CHART_MARGIN.top}
                      y2={CHART_MARGIN.top + innerHeight}
                      vectorEffect="non-scaling-stroke"
                    />
                  ) : null}

                  {historyPoints.length > 1 ? (
                    <polyline
                      className="history-line"
                      fill="none"
                      points={linePoints}
                    />
                  ) : null}

                  {chartPoints.map((point) => {
                    const isActive = point.key === activePoint?.key;

                    return (
                      <g key={point.key}>
                        <circle
                          className="history-point-hit"
                          cx={point.chartX}
                          cy={point.chartY}
                          r={14}
                          onMouseEnter={() => setHoveredPointKey(point.key)}
                        />
                        {isActive ? (
                          <circle
                            className="history-point-glow"
                            cx={point.chartX}
                            cy={point.chartY}
                            r={ACTIVE_POINT_RADIUS + 5}
                          />
                        ) : null}
                        <circle
                          className={`history-point${isActive ? ' history-point-active' : ''}`}
                          cx={point.chartX}
                          cy={point.chartY}
                          r={isActive ? ACTIVE_POINT_RADIUS : POINT_RADIUS}
                          tabIndex={0}
                          aria-label={`${point.playedAtLabel ?? formatPointTime(point.playedAtUnix, locale)} ${formatPercent(point.achievementPercent)}`}
                          onMouseEnter={() => setHoveredPointKey(point.key)}
                          onFocus={() => setHoveredPointKey(point.key)}
                          onBlur={() => setHoveredPointKey((current) => (current === point.key ? null : current))}
                        />
                      </g>
                    );
                  })}
                </svg>
              </div>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
