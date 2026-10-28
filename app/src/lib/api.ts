// The daemon's local API, as the interface uses it. The native side says where it is and which
// token to send; every call carries the token.
import { invoke } from '@tauri-apps/api/core'

export type Category = 'reply_today' | 'informative' | 'newsletter' | 'invoice'

export const CATEGORIES: { key: Category; label: string; hint: string }[] = [
  { key: 'reply_today', label: 'Reply today', hint: 'Someone is waiting for you' },
  { key: 'informative', label: 'Informative', hint: 'Worth knowing, nothing to do' },
  { key: 'newsletter', label: 'Newsletters', hint: 'Bulk mail you subscribed to' },
  { key: 'invoice', label: 'Invoices', hint: 'Bills, receipts, payments' },
]

export interface Brief {
  id: number
  thread_id: string
  from: string
  from_name: string
  to: string[]
  cc: string[]
  subject: string
  date: number
  lang: string
  seen: boolean
  outgoing: boolean
  snippet: string
  attachments: string[]
  list_unsubscribe: boolean
  category?: Category | null
  reason?: string | null
  label?: Category | null
}

export interface Message {
  id: number
  thread_id: string
  from_addr: string
  from_name: string
  to: string[]
  cc: string[]
  subject: string
  date: number
  body: string
  lang: string
  attachments: string[]
  outgoing: boolean
}

export interface Draft {
  id: number
  reply_to: number
  to: string[]
  subject: string
  body: string
  status: 'proposed' | 'saved' | 'sent' | 'discarded'
  created: number
}

export interface Summary {
  summary: string
  decisions: string[]
  open_questions: string[]
  waiting_on: string
}

export interface Detail {
  message: Message
  thread: { brief: Brief; body: string }[]
  summary: { messages: number; summary: Summary } | null
  drafts: Draft[]
  category: Category | null
  reason: string | null
  label: Category | null
}

export interface FollowUp {
  id: number
  message_id: number
  thread_id: string
  subject: string
  waiting_on: string
  what: string
  sent: number
  due: number
  status: string
  overdue: boolean
}

export interface Account {
  id: number
  name: string
  email: string
  imap_host: string
  imap_port: number
  imap_security: string
  smtp_host: string
  smtp_port: number
  smtp_security: string
  username: string
}

export interface Status {
  status: { syncing: boolean; last_sync: number; last_error: string | null; triaged: number; model: string; model_ready: boolean }
  counts: { messages: number; incoming: number; untriaged: number; labelled: number; followups_due: number; by_category: Record<Category, number> }
}

let conn: Promise<{ url: string; token: string; data_dir: string }> | null = null

export function connect() {
  conn ??= invoke<{ url: string; token: string; data_dir: string }>('connection')
  conn.catch(() => (conn = null))
  return conn
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const { url, token } = await connect()
  const r = await fetch(url + path, {
    method,
    headers: { Authorization: `Bearer ${token}`, ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}) },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  const text = await r.text()
  const data = text ? JSON.parse(text) : null
  if (!r.ok) throw new Error(data?.error ?? `${r.status} ${r.statusText}`)
  return data as T
}

export const api = {
  status: () => call<Status>('GET', '/api/status'),
  inbox: (category?: Category, limit = 300) =>
    call<Brief[]>('GET', `/api/inbox?limit=${limit}${category ? `&category=${category}` : ''}`),
  search: (q: string) => call<Brief[]>('GET', `/api/search?q=${encodeURIComponent(q)}`),
  message: (id: number) => call<Detail>('GET', `/api/messages/${id}`),
  label: (id: number, category: Category | null) => call('POST', `/api/messages/${id}/label`, { category }),
  nextUnlabelled: () => call<{ message: Message | null; labelled: number }>('GET', '/api/label/next'),
  draft: (id: number) => call<Draft>('POST', `/api/messages/${id}/draft`),
  editDraft: (id: number, body: string) => call<Draft>('POST', `/api/drafts/${id}`, { body }),
  saveDraft: (id: number) => call<Draft>('POST', `/api/drafts/${id}/save`),
  sendDraft: (id: number, confirmation: string) => call<Draft>('POST', `/api/drafts/${id}/send`, { confirmation }),
  discardDraft: (id: number) => call('POST', `/api/drafts/${id}/discard`),
  summarize: (thread: string) => call<{ messages: number; summary: Summary }>('POST', `/api/threads/${encodeURIComponent(thread)}/summarize`),
  followups: (status?: string) => call<FollowUp[]>('GET', `/api/followups${status ? `?status=${status}` : ''}`),
  followupAction: (id: number, action: 'dismiss' | 'done' | 'snooze', days?: number) =>
    call('POST', `/api/followups/${id}`, { action, days }),
  accounts: () => call<Account[]>('GET', '/api/accounts'),
  addAccount: (a: Omit<Account, 'id'> & { password: string }) => call<{ id: number }>('POST', '/api/accounts', { id: 0, ...a }),
  deleteAccount: (id: number) => call('POST', `/api/accounts/${id}/delete`),
  sync: () => call('POST', '/api/sync'),
  style: () =>
    call<{ account: string; style: { messages: number; median_words: number; diacritics: number; by_lang: Record<string, { messages: number; greetings: string[]; signoffs: string[] }> } }[]>('GET', '/api/style'),
}

/** The confirmation the daemon requires to send a draft: its id and its recipients, as shown. */
export const confirmationFor = (d: Draft) => `${d.id}:${d.to.join(',')}`

export function when(t: number): string {
  const d = new Date(t * 1000)
  const now = new Date()
  if (d.toDateString() === now.toDateString()) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  const days = (now.getTime() - d.getTime()) / 86400000
  if (days < 6) return d.toLocaleDateString([], { weekday: 'short' })
  return d.toLocaleDateString([], { day: 'numeric', month: 'short', ...(d.getFullYear() !== now.getFullYear() ? { year: 'numeric' } : {}) })
}

export const sender = (b: { from_name?: string; from?: string; from_addr?: string }) => b.from_name || b.from || b.from_addr || ''
