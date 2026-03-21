import { useState } from 'react';

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

const CHART_WIDTH = 820;
const CHART_HEIGHT = 360;
const CHART_MARGIN = { top: 28, right: 32, bottom: 96, left: 96 };
const POINT_RADIUS = 5;

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
  const linePoints = chartPoints
    .map((point) => `${point.chartX},${point.chartY}`)
    .join(' ');
  const hoveredPoint = chartPoints.find((point) => point.key === hoveredPointKey) ?? null;

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
              {hoveredPoint ? (
                <div
                  className="history-tooltip"
                  style={{
                    left: `${(hoveredPoint.chartX / CHART_WIDTH) * 100}%`,
                    top: `${(hoveredPoint.chartY / CHART_HEIGHT) * 100}%`,
                  }}
                >
                  <strong>{formatPercent(hoveredPoint.achievementPercent)}</strong>
                  <span>{hoveredPoint.playedAtLabel ?? formatPointTime(hoveredPoint.playedAtUnix, locale)}</span>
                </div>
              ) : null}

              <div className="history-chart-scroll">
                <svg
                  viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
                  className="history-chart"
                  role="img"
                  aria-label={t('history.graphLabel', { title: selectedHistoryRow.title })}
                >
                  {yTicks.map((tick) => (
                    <g key={`y-${tick}`} className="history-grid">
                      <line
                        x1={CHART_MARGIN.left}
                        x2={CHART_MARGIN.left + innerWidth}
                        y1={toChartY(tick)}
                        y2={toChartY(tick)}
                      />
                      <text
                        x={CHART_MARGIN.left - 10}
                        y={toChartY(tick)}
                        textAnchor="end"
                        dominantBaseline="middle"
                      >
                        {tick.toFixed(4)}%
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
                  />
                  <line
                    className="history-axis"
                    x1={CHART_MARGIN.left}
                    x2={CHART_MARGIN.left + innerWidth}
                    y1={CHART_MARGIN.top + innerHeight}
                    y2={CHART_MARGIN.top + innerHeight}
                  />

                  <text
                    className="history-axis-label"
                    x={26}
                    y={CHART_MARGIN.top + innerHeight / 2}
                    transform={`rotate(-90 26 ${CHART_MARGIN.top + innerHeight / 2})`}
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

                  {historyPoints.length > 1 ? (
                    <polyline
                      className="history-line"
                      fill="none"
                      points={linePoints}
                    />
                  ) : null}

                  {chartPoints.map((point) => (
                    <g key={point.key}>
                      <circle
                        className="history-point"
                        cx={point.chartX}
                        cy={point.chartY}
                        r={POINT_RADIUS}
                        tabIndex={0}
                        aria-label={`${point.playedAtLabel ?? formatPointTime(point.playedAtUnix, locale)} ${formatPercent(point.achievementPercent)}`}
                        onMouseEnter={() => setHoveredPointKey(point.key)}
                        onFocus={() => setHoveredPointKey(point.key)}
                        onBlur={() => setHoveredPointKey((current) => (current === point.key ? null : current))}
                      />
                    </g>
                  ))}
                </svg>
              </div>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
