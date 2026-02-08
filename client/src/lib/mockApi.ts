export type ExecutionPoint = {
  bucketTsUnix: number
  avgBps: number
}

export type ExecutionCostResponse = {
  data: ExecutionPoint[]
}

export type FetchParams = {
  symbol: string
  from: number
  to: number
  quoteAmount: number
}

const HOUR_MS = 60 * 60 * 1000

const seedFromString = (value: string) => {
  let hash = 0
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash << 5) - hash + value.charCodeAt(i)
    hash |= 0
  }
  return Math.abs(hash)
}

const seededRandom = (seed: number) => {
  let current = seed
  return () => {
    current = (current * 9301 + 49297) % 233280
    return current / 233280
  }
}

const buildHourlySeries = ({
  symbol,
  from,
  to,
  quoteAmount,
}: FetchParams): ExecutionPoint[] => {
  const hours = Math.max(1, Math.floor((to - from) / HOUR_MS) + 1)
  const rand = seededRandom(seedFromString(`${symbol}-${quoteAmount}`))
  const quoteAmountBtc = quoteAmount / 100_000_000
  const base = 0.015 + quoteAmountBtc * 0.22
  const symbolBump = (seedFromString(symbol) % 7) * 0.0015
  const points: ExecutionPoint[] = []

  for (let i = 0; i < hours; i += 1) {
    const timestampMs = from + i * HOUR_MS
    const wave = Math.sin(i / 5) * 0.01
    const noise = (rand() - 0.5) * 0.03
    const slippagePct = Math.max(0.004, base + symbolBump + wave + noise)
    points.push({
      bucketTsUnix: Math.floor(timestampMs / 1000),
      avgBps: slippagePct * 100,
    })
  }

  return points
}

export const fetchHourlyExecutionCost = async (
  params: FetchParams,
): Promise<ExecutionCostResponse> => {
  const data = buildHourlySeries(params)
  await new Promise((resolve) => setTimeout(resolve, 150))
  return { data }
}

