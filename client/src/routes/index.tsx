import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ExecutionChart } from '../components/ExecutionChart'
import { fetchHourlyExecutionCost } from '../lib/executionApi'
import { Button } from '../components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../components/ui/card'
import { Input } from '../components/ui/input'
import { Label } from '../components/ui/label'
import { Switch } from '../components/ui/switch'
import { ToggleGroup, ToggleGroupItem } from '../components/ui/toggle-group'

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
  const [now, setNow] = useState(() => Date.now())
  const selectedRange =
    TIME_RANGES.find((range) => range.hours === rangeHours) ?? TIME_RANGES[0]

  const refreshNow = () => setNow(Date.now())

  const { from, to } = useMemo(() => {
    const end = now
    return { from: end - rangeHours * HOUR_MS, to: end }
  }, [now, rangeHours])

  const {
    data: executionResponse,
    isFetching,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ['executionCost', symbol, quoteAmount, rangeHours],
    queryFn: () => fetchHourlyExecutionCost({ symbol, quoteAmount, from, to }),
  })

  const executionPoints = useMemo(
    () => executionResponse?.data ?? [],
    [executionResponse?.data],
  )

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
    <div className="flex w-full max-w-[1440px] flex-col gap-8">
      <header className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-[0.28em] text-accent/90">
            Roxom Markets
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl lg:text-[44px]">
            Roxom Liquid View
          </h1>
          <p className="mt-3 max-w-xl text-base text-muted-foreground">
            Real Execution cost for Roxom perpetuals.
          </p>
          <div className="mt-5 grid gap-2 rounded-2xl border border-border/30 bg-[linear-gradient(135deg,rgba(24,18,40,0.9),rgba(12,10,20,0.7))] px-4 py-3 text-sm text-zinc-200 shadow-[0_12px_24px_rgba(8,6,14,0.45)]">
            <div className="text-xs font-semibold uppercase tracking-[0.24em] text-accent/90">
              Execution cost explained
            </div>
            <ul className="grid list-disc gap-2 pl-4 text-[15px] text-zinc-300">
              <li>
                This chart shows the average execution cost for market orders at
                the selected size and time range.
              </li>
              <li>
                For each size, we simulate orders and measure bid/ask slippage
                against the mid price at each orderbook snapshot, then average
                the results.
              </li>
              <li>Account fees are not included in these slippage values.</li>
            </ul>
          </div>
        </div>
      </header>

      <Card className="border-border/20 bg-card/90 p-6 sm:p-8">
        <CardHeader className="space-y-2 pb-4">
          <CardTitle className="text-2xl sm:text-3xl">{symbol}</CardTitle>
          <CardDescription className="text-base text-muted-foreground sm:text-lg">
            {formatQuoteAmount(quoteAmount, quoteUnit)} market order ·{' '}
            {selectedRange.description}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-5">
            <div className="flex flex-col gap-3">
              <Label>Symbol</Label>
              <ToggleGroup
                type="single"
                value={symbol}
                onValueChange={(value: string) => {
                  if (!value) {
                    return
                  }
                  setSymbol(value)
                  refreshNow()
                }}
              >
                {SYMBOLS.map((item) => (
                  <ToggleGroupItem key={item} value={item}>
                    {item}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </div>

            <div className="flex flex-col gap-3">
              <Label>Order value ({quoteUnit})</Label>
              <ToggleGroup
                type="single"
                value={String(quoteAmount)}
                onValueChange={(value: string) => {
                  if (!value) {
                    return
                  }
                  setQuoteAmount(Number(value))
                  refreshNow()
                }}
              >
                {QUOTE_AMOUNTS.map((amount) => (
                  <ToggleGroupItem key={amount} value={String(amount)}>
                    {formatQuoteChip(amount, quoteUnit)}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </div>

            <div className="flex flex-col gap-3">
              <Label>Unit</Label>
              <ToggleGroup
                type="single"
                value={quoteUnit}
                onValueChange={(value: string) =>
                  value && setQuoteUnit(value as QuoteUnit)
                }
              >
                {QUOTE_UNITS.map((unit) => (
                  <ToggleGroupItem key={unit} value={unit}>
                    {unit}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </div>

            <div className="flex flex-col gap-3">
              <Label>Range</Label>
              <ToggleGroup
                type="single"
                value={String(rangeHours)}
                onValueChange={(value: string) => {
                  if (!value) {
                    return
                  }
                  setRangeHours(Number(value))
                  refreshNow()
                }}
              >
                {TIME_RANGES.map((range) => (
                  <ToggleGroupItem key={range.label} value={String(range.hours)}>
                    {range.label}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </div>

            <div className="flex flex-col gap-3">
              <Label>Slippage unit</Label>
              <ToggleGroup
                type="single"
                value={slippageUnit}
                onValueChange={(value: string) =>
                  value && setSlippageUnit(value as SlippageUnit)
                }
              >
                {SLIPPAGE_UNITS.map((unit) => (
                  <ToggleGroupItem key={unit} value={unit}>
                    {unit}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
            </div>

            <div className="flex flex-col gap-3">
              <Label>Taker fee</Label>
              <div className="flex flex-wrap items-center gap-4">
                <div className="flex items-center gap-2 text-sm">
                  <Switch
                    checked={takerFeeEnabled}
                    onCheckedChange={setTakerFeeEnabled}
                    id="taker-fee-toggle"
                  />
                  <label htmlFor="taker-fee-toggle">Apply fee</label>
                </div>
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    min={0}
                    step={0.01}
                    value={takerFeePct}
                    onChange={(event) =>
                      setTakerFeePct(Number(event.target.value))
                    }
                    className="h-10 w-24"
                  />
                  <span className="text-sm text-muted-foreground">%</span>
                </div>
              </div>
            </div>
          </div>

          <div className="rounded-2xl border border-border/20 bg-card/60 p-5 shadow-[0_18px_36px_rgba(8,6,14,0.45)]">
            <div className="flex flex-wrap items-center gap-5">
              <div className="flex flex-col gap-1 text-left">
                <span className="text-xs uppercase tracking-wide text-muted-foreground">
                  Min
                </span>
                <span className="text-lg font-semibold">
                  {formatSlippage(rangeStats.min)}
                </span>
              </div>
              <div className="flex flex-col gap-1 text-left">
                <span className="text-xs uppercase tracking-wide text-muted-foreground">
                  Avg
                </span>
                <span className="text-lg font-semibold">
                  {formatSlippage(rangeStats.avg)}
                </span>
              </div>
              <div className="flex flex-col gap-1 text-left">
                <span className="text-xs uppercase tracking-wide text-muted-foreground">
                  Max
                </span>
                <span className="text-lg font-semibold">
                  {formatSlippage(rangeStats.max)}
                </span>
              </div>
            </div>
            <div className="mt-4 h-[clamp(420px,55vh,760px)]">
              {isFetching ? (
                <div className="grid h-full place-items-center text-sm text-muted-foreground">
                  Loading hourly averages…
                </div>
              ) : isError ? (
                <div className="grid h-full place-items-center gap-3 text-center">
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-foreground">
                      Failed to load execution data.
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {error instanceof Error
                        ? error.message
                        : 'Please try again in a moment.'}
                    </p>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => refetch()}
                  >
                    Retry
                  </Button>
                </div>
              ) : !totalPoints.length ? (
                <div className="grid h-full place-items-center text-sm text-muted-foreground">
                  No data for this range.
                </div>
              ) : (
                <ExecutionChart points={totalPoints} slippageUnit={slippageUnit} />
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      <footer className="flex flex-col gap-2 border-t border-border/20 pt-5 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
        <span>Roxom perpetuals · execution cost analytics.</span>
        <span>Built for fast, simple market insight.</span>
      </footer>
    </div>
  )
}

