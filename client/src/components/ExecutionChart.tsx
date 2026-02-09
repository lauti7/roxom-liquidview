import { useMemo } from 'react'
import * as echarts from 'echarts'
import ReactECharts from 'echarts-for-react'

const getCssVar = (name: string, fallback: string) => {
  if (typeof window === 'undefined') {
    return fallback
  }
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim()
  return value || fallback
}

const hslToRgba = (value: string, alpha = 1) => {
  const parts = value.replace(/%/g, '').split(/\s+/).filter(Boolean)
  if (parts.length < 3) {
    return `rgba(0, 0, 0, ${alpha})`
  }
  const [hRaw, sRaw, lRaw] = parts
  const h = ((Number(hRaw) % 360) + 360) % 360
  const s = Number(sRaw) / 100
  const l = Number(lRaw) / 100
  const c = (1 - Math.abs(2 * l - 1)) * s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = l - c / 2

  let r = 0
  let g = 0
  let b = 0

  if (h < 60) {
    r = c
    g = x
  } else if (h < 120) {
    r = x
    g = c
  } else if (h < 180) {
    g = c
    b = x
  } else if (h < 240) {
    g = x
    b = c
  } else if (h < 300) {
    r = x
    b = c
  } else {
    r = c
    b = x
  }

  const toRgb = (channel: number) => Math.round((channel + m) * 255)
  return `rgba(${toRgb(r)}, ${toRgb(g)}, ${toRgb(b)}, ${alpha})`
}

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
      const lineHsl = getCssVar('--chart-line', '270.7 91% 65.1%')
      const gridHsl = getCssVar('--chart-grid', '251.4 100% 74.1%')
      const axisHsl = getCssVar('--chart-axis', '240 5% 64.9%')
      const tooltipHsl = getCssVar('--chart-tooltip', '255 33.3% 4.7%')
      const foregroundHsl = getCssVar('--foreground', '258 100% 98%')
      const lineColor = hslToRgba(lineHsl, 1)
      const gridLineColor = hslToRgba(gridHsl, 0.35)
      const gridSplitColor = hslToRgba(gridHsl, 0.18)
      const axisColor = hslToRgba(axisHsl, 1)
      const tooltipColor = hslToRgba(tooltipHsl, 0.92)
      const tooltipBorder = hslToRgba(lineHsl, 0.6)
      const tooltipText = hslToRgba(foregroundHsl, 1)
      const tooltipPointer = hslToRgba(lineHsl, 0.85)

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
          backgroundColor: tooltipColor,
          borderColor: tooltipBorder,
          borderWidth: 1,
          textStyle: {
            color: tooltipText,
            fontSize: 12,
          },
          axisPointer: {
            type: 'cross',
            label: {
              backgroundColor: tooltipPointer,
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
              `<span style="color:${axisColor};">${dateLabel}</span>`,
              `<strong style="font-size:14px;">${formatValue(value)}</strong>`,
              `</div>`,
            ].join('')
          },
        },
        xAxis: {
          type: 'time',
          axisLine: {
            lineStyle: { color: gridLineColor },
          },
          axisTick: { show: false },
          axisLabel: {
            color: axisColor,
            fontSize: 11,
          },
          splitLine: { show: false },
        },
        yAxis: {
          type: 'value',
          axisLine: { show: false },
          axisTick: { show: false },
          axisLabel: {
            color: axisColor,
            formatter: (value: number) => formatValue(value),
            fontSize: 11,
          },
          splitLine: {
            lineStyle: {
              color: gridSplitColor,
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
              color: lineColor,
            },
            areaStyle: {
              color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
                { offset: 0, color: hslToRgba(lineHsl, 0.35) },
                { offset: 1, color: hslToRgba(lineHsl, 0) },
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

