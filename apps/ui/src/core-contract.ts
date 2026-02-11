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
