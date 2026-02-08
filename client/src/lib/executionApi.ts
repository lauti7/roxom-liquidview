import {
  type ExecutionCostResponse,
  type ExecutionPoint,
  type FetchParams,
  fetchHourlyExecutionCost as fetchMockExecutionCost,
} from './mockApi'

const API_BASE_URL = import.meta.env.VITE_EXECUTION_API_URL

type ExecutionCostPayload = {
  data: ExecutionPoint[]
}

const fetchRealExecutionCost = async (
  params: FetchParams,
): Promise<ExecutionCostResponse> => {
  const url = new URL('/liquidity', API_BASE_URL)
  url.searchParams.set('symbol', params.symbol)
  url.searchParams.set('amount', String(params.quoteAmount))
  url.searchParams.set('from', String(Math.floor(params.from / 1000)))
  url.searchParams.set('to', String(Math.floor(params.to / 1000)))

  const response = await fetch(url.toString())
  if (!response.ok) {
    throw new Error('Failed to fetch execution cost')
  }

  const payload = (await response.json()) as ExecutionCostPayload
  if (!payload?.data || !Array.isArray(payload.data)) {
    throw new Error('Invalid execution cost payload')
  }

  return payload
}

export const fetchHourlyExecutionCost = async (
  params: FetchParams,
): Promise<ExecutionCostResponse> => {
  if (!API_BASE_URL) {
    return fetchMockExecutionCost(params)
  }

  return fetchRealExecutionCost(params)
}

export type { ExecutionCostResponse, ExecutionPoint, FetchParams }
