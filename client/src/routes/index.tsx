import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ExecutionChart } from '../components/ExecutionChart'
import { fetchHourlyExecutionCost } from '../lib/mockApi'

const SYMBOLS = ['GOLD-BTC', 'US500-BTC']

const QUOTE_AMOUNTS = [250_000, 500_000, 1_000_000, 10_000_000, 50_000_000]

const TIME_RANGES = [
  { label: '6H', hours: 6, description: 'last 6 hours' },
  { label: '24H', hours: 24, description: 'last 24 hours' },
  { label: '7D', hours: 24 * 7, description: 'last 7 days' },
]

const HOUR_MS = 60 * 60 * 1000

const SATS_PER_BTC = 100_000_000
const QUOTE_UNITS = ['SATS', 'BTC'] as const
type QuoteUnit = (typeof QUOTE_UNITS)[number]
const SLIPPAGE_UNITS = ['BPS', '%'] as const
type SlippageUnit = (typeof SLIPPAGE_UNITS)[number]

const trimTrailingZeros = (value: string) => value.replace(/\.?0+$/, '')
const formatSats = (value: number) =>
  value.toLocaleString(undefined, { maximumFractionDigits: 0 })
const formatSatsCompact = (value: number) => {
  if (value >= 1_000_000) {
    return `${trimTrailingZeros((value / 1_000_000).toFixed(2))}m`
  }

  if (value >= 1_000) {
    return `${trimTrailingZeros((value / 1_000).toFixed(2))}k`
  }

  return `${value}`
}
const formatBtc = (value: number) =>
  trimTrailingZeros((value / SATS_PER_BTC).toFixed(8))
const formatQuoteAmount = (value: number, unit: QuoteUnit) =>
  unit === 'BTC' ? `${formatBtc(value)} BTC` : `${formatSats(value)} sats`
const formatQuoteChip = (value: number, unit: QuoteUnit) =>
  unit === 'BTC' ? formatBtc(value) : formatSatsCompact(value)

export function IndexPage() {
  const [symbol, setSymbol] = useState(SYMBOLS[0])
  const [quoteAmount, setQuoteAmount] = useState(QUOTE_AMOUNTS[1])
  const [quoteUnit, setQuoteUnit] = useState<QuoteUnit>('SATS')
  const [slippageUnit, setSlippageUnit] = useState<SlippageUnit>('BPS')
  const [rangeHours, setRangeHours] = useState(TIME_RANGES[0].hours)
  const [takerFeePct, setTakerFeePct] = useState(0.05)
  const [takerFeeEnabled, setTakerFeeEnabled] = useState(true)
  const selectedRange =
    TIME_RANGES.find((range) => range.hours === rangeHours) ?? TIME_RANGES[0]

  const { from, to } = useMemo(() => {
    const end = Date.now()
    return { from: end - rangeHours * HOUR_MS, to: end }
  }, [rangeHours])

  const { data: executionResponse, isFetching } = useQuery({
    queryKey: ['executionCost', symbol, quoteAmount, rangeHours],
    queryFn: () => fetchHourlyExecutionCost({ symbol, quoteAmount, from, to }),
  })

  const executionPoints = executionResponse?.data ?? []

  const totalPoints = useMemo(
    () =>
      executionPoints.map((point) => {
        const totalCostValue =
          slippageUnit === 'BPS'
            ? point.avgBps + (takerFeeEnabled ? takerFeePct * 100 : 0)
            : point.avgBps / 100 + (takerFeeEnabled ? takerFeePct : 0)

        return {
          timestamp: point.bucketTsUnix * 1000,
          totalCostValue,
          totalCostPct: totalCostValue,
        }
      }),
    [executionPoints, slippageUnit, takerFeeEnabled, takerFeePct],
  )

  const rangeStats = useMemo(() => {
    if (!totalPoints.length) {
      return { min: null, avg: null, max: null }
    }

    const values = totalPoints.map((point) => point.totalCostValue)
    const min = Math.min(...values)
    const max = Math.max(...values)
    const avg = values.reduce((sum, value) => sum + value, 0) / values.length
    return { min, avg, max }
  }, [totalPoints])

  const formatSlippage = (value: number | null) => {
    if (value === null) {
      return '—'
    }
    return slippageUnit === 'BPS'
      ? `${value.toFixed(3)} bps`
      : `${value.toFixed(3)}%`
  }

  return (
    <div className="page">
      <header className="header">
        <div>
          <p className="eyebrow">Roxom Markets</p>
          <h1>Roxom Liquid View</h1>
          <p className="subheading">
            Real Execution cost for Roxom perpetuals.
          </p>
          <div className="panel-explainer">
            <div className="panel-explainer-title">Execution cost explained</div>
            <ul className="panel-explainer-list">
              <li>
                This chart shows the average execution cost for market orders at
                the selected size and time range.
              </li>
              <li>
                For each size, we simulate orders and measure bid/ask slippage
                against the mid price at each order book snapshot, then average
                the results.
              </li>
              <li>Account fees are not included in these slippage values.</li>
            </ul>
          </div>
        </div>
      </header>

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>{symbol}</h2>
            <p className="muted">
              {formatQuoteAmount(quoteAmount, quoteUnit)} market order ·{' '}
              {selectedRange.description}
            </p>
          </div>
        </div>
        <div className="controls">
          <div className="control-group">
            <span className="control-label">Symbol</span>
            <div className="chip-group">
              {SYMBOLS.map((item) => (
                <button
                  key={item}
                  type="button"
                  className={`chip ${item === symbol ? 'active' : ''}`}
                  onClick={() => setSymbol(item)}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">
              Quote amount ({quoteUnit})
            </span>
            <div className="chip-group">
              {QUOTE_AMOUNTS.map((amount) => (
                <button
                  key={amount}
                  type="button"
                  className={`chip ${
                    amount === quoteAmount ? 'active' : ''
                  }`}
                  onClick={() => setQuoteAmount(amount)}
                >
                  {formatQuoteChip(amount, quoteUnit)}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">Unit</span>
            <div className="chip-group">
              {QUOTE_UNITS.map((unit) => (
                <button
                  key={unit}
                  type="button"
                  className={`chip ${unit === quoteUnit ? 'active' : ''}`}
                  onClick={() => setQuoteUnit(unit)}
                >
                  {unit}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">Range</span>
            <div className="chip-group">
              {TIME_RANGES.map((range) => (
                <button
                  key={range.label}
                  type="button"
                  className={`chip ${
                    range.hours === rangeHours ? 'active' : ''
                  }`}
                  onClick={() => setRangeHours(range.hours)}
                >
                  {range.label}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">Slippage unit</span>
            <div className="chip-group">
              {SLIPPAGE_UNITS.map((unit) => (
                <button
                  key={unit}
                  type="button"
                  className={`chip ${unit === slippageUnit ? 'active' : ''}`}
                  onClick={() => setSlippageUnit(unit)}
                >
                  {unit}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">Taker fee</span>
            <div className="fee-control">
              <label className="toggle">
                <input
                  type="checkbox"
                  checked={takerFeeEnabled}
                  onChange={(event) => setTakerFeeEnabled(event.target.checked)}
                />
                <span>Apply fee</span>
              </label>
              <input
                type="number"
                min={0}
                step={0.01}
                value={takerFeePct}
                onChange={(event) =>
                  setTakerFeePct(Number(event.target.value))
                }
              />
              <span className="muted">%</span>
            </div>
          </div>
        </div>

        <div className="chart-card">
          <div className="stats-row">
            <div className="stat">
              <span className="stat-label">Min</span>
              <span className="stat-value">
                {formatSlippage(rangeStats.min)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Avg</span>
              <span className="stat-value">
                {formatSlippage(rangeStats.avg)}
              </span>
            </div>
            <div className="stat">
              <span className="stat-label">Max</span>
              <span className="stat-value">
                {formatSlippage(rangeStats.max)}
              </span>
            </div>
          </div>
          <div className="chart-area">
            {isFetching ? (
              <div className="chart-loading">Loading hourly averages…</div>
            ) : (
              <ExecutionChart points={totalPoints} slippageUnit={slippageUnit} />
            )}
          </div>
        </div>
      </section>

      <footer className="footer">
        <span>Roxom perpetuals · execution cost analytics.</span>
        <span>Built for fast, simple market insight.</span>
      </footer>
    </div>
  )
}

