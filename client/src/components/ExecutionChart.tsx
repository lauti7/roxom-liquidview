import { useMemo } from 'react'
import * as echarts from 'echarts'
import ReactECharts from 'echarts-for-react'

export type ExecutionChartPoint = {
  timestamp: number
  totalCostValue: number
  totalCostPct: number
}

type ExecutionChartProps = {
  points: ExecutionChartPoint[]
  slippageUnit: 'BPS' | '%'
}

export function ExecutionChart({ points, slippageUnit }: ExecutionChartProps) {
  const seriesData = useMemo(
    () => points.map((point) => [point.timestamp, point.totalCostValue]),
    [points],
  )

  const options = useMemo(
    () => {
      const unitSuffix = slippageUnit === 'BPS' ? ' bps' : '%'
      const formatValue = (value: number) => `${value.toFixed(3)}${unitSuffix}`
      const seriesName =
        slippageUnit === 'BPS' ? 'Execution Cost (BPS)' : 'Execution Cost (%)'

      return {
      backgroundColor: 'transparent',
      grid: {
        left: 12,
        right: 16,
        top: 16,
        bottom: 28,
        containLabel: true,
      },
      tooltip: {
        trigger: 'axis',
        backgroundColor: 'rgba(10, 8, 16, 0.92)',
        borderColor: 'rgba(168, 85, 247, 0.6)',
        borderWidth: 1,
        textStyle: {
          color: '#f8f5ff',
          fontSize: 12,
        },
        axisPointer: {
          type: 'cross',
          label: {
            backgroundColor: 'rgba(168, 85, 247, 0.85)',
            borderRadius: 6,
          },
        },
        formatter: (params: Array<{ value: [number, number] }>) => {
          if (!params.length) {
            return ''
          }
          const [timestamp, value] = params[0].value
          const dateLabel = new Date(timestamp).toLocaleString(undefined, {
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
          })
          return [
            `<div style="display:flex;flex-direction:column;gap:6px;">`,
            `<span style="color:#a1a1aa;">${dateLabel}</span>`,
            `<strong style="font-size:14px;">${formatValue(value)}</strong>`,
            `</div>`,
          ].join('')
        },
      },
      xAxis: {
        type: 'time',
        axisLine: {
          lineStyle: { color: 'rgba(148, 123, 255, 0.35)' },
        },
        axisTick: { show: false },
        axisLabel: {
          color: '#a1a1aa',
          fontSize: 11,
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'value',
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          color: '#a1a1aa',
          formatter: (value: number) => formatValue(value),
          fontSize: 11,
        },
        splitLine: {
          lineStyle: {
            color: 'rgba(148, 123, 255, 0.18)',
          },
        },
      },
      series: [
        {
          name: seriesName,
          type: 'line',
          data: seriesData,
          smooth: true,
          showSymbol: false,
          lineStyle: {
            width: 2,
            color: '#a855f7',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(168, 85, 247, 0.35)' },
              { offset: 1, color: 'rgba(168, 85, 247, 0)' },
            ]),
          },
        },
      ],
      }
    },
    [seriesData, slippageUnit],
  )

  return (
    <ReactECharts
      option={options}
      style={{ width: '100%', height: '100%' }}
      notMerge
      lazyUpdate
    />
  )
}

