export type SearchRequest = {
  query: string
  limit?: number
}

export type SearchResultDto = {
  id: string
  kind: string
  title: string
  path: string
}

export type SearchResponse = {
  results: SearchResultDto[]
}

export type LaunchRequest = {
  id?: string
  path?: string
}

export type LaunchResponse = {
  launched: boolean
}

export type CoreRequest =
  | { kind: 'Search'; payload: SearchRequest }
  | { kind: 'Launch'; payload: LaunchRequest }

export type CoreResponse =
  | { kind: 'Search'; payload: SearchResponse }
  | { kind: 'Launch'; payload: LaunchResponse }

export type ErrorCode =
  | 'invalid_json'
  | 'invalid_request'
  | 'item_not_found'
  | 'launch'
  | 'store'
  | 'config'
  | 'provider'

export type ErrorResponse = {
  code: ErrorCode
  message: string
}

export type TransportResponse =
  | { status: 'ok'; response: CoreResponse }
  | { status: 'err'; error: ErrorResponse }
