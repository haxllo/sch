import type {
  CoreRequest,
  CoreResponse,
  LaunchRequest,
  SearchResultDto,
  TransportResponse,
} from './core-contract'

declare global {
  interface Window {
    swiftfindCore?: {
      invoke: (request: CoreRequest) => Promise<TransportResponse>
    }
  }
}

function bridgeUnavailableError(): Error {
  return new Error('Core bridge unavailable')
}

async function send(request: CoreRequest): Promise<CoreResponse> {
  const bridge = window.swiftfindCore
  if (!bridge) {
    throw bridgeUnavailableError()
  }

  const response = await bridge.invoke(request)
  if (response.status === 'err') {
    throw new Error(response.error.message)
  }

  return response.response
}

export async function searchCommand(
  query: string,
  limit: number,
): Promise<SearchResultDto[]> {
  const response = await send({
    kind: 'Search',
    payload: { query, limit },
  })

  if (response.kind !== 'Search') {
    throw new Error('Unexpected response kind for search command')
  }

  return response.payload.results
}

export async function launchCommand(payload: LaunchRequest): Promise<void> {
  const response = await send({
    kind: 'Launch',
    payload,
  })

  if (response.kind !== 'Launch') {
    throw new Error('Unexpected response kind for launch command')
  }

  if (!response.payload.launched) {
    throw new Error('Launch command returned unsuccessful state')
  }
}
