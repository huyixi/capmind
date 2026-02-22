export interface Memo {
  id: string
  clientId?: string
  user_id: string
  text: string
  images: string[]
  imageCount?: number
  created_at: string
  updated_at: string
  version: string
  deleted_at?: string | null
  serverVersion?: string
  hasConflict?: boolean
  conflictServerMemo?: Memo
  conflictType?: "update" | "delete" | "restore"
  hasImages?: boolean
}

export interface User {
  id: string
  email: string
}
