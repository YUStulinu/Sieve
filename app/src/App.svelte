<script lang="ts">
  import { api, CATEGORIES, connect, sender, when, type Brief, type Category, type Status } from './lib/api'
  import FollowUps from './lib/FollowUps.svelte'
  import LabelMode from './lib/LabelMode.svelte'
  import MessageView from './lib/MessageView.svelte'
  import Settings from './lib/Settings.svelte'

  type View = Category | 'all' | 'followups' | 'label' | 'settings'

  let view = $state<View>('reply_today')
  let list = $state<Brief[]>([])
  let selected = $state<number | null>(null)
  let status = $state<Status | null>(null)
  let query = $state('')
  let error = $state('')
  let starting = $state(true)

  async function refresh() {
    try {
      status = await api.status()
      if (view === 'all' || CATEGORIES.some((c) => c.key === view)) {
        list = query.trim() ? await api.search(query) : await api.inbox(view === 'all' ? undefined : (view as Category))
      }
      error = ''
    } catch (e) {
      error = String(e)
    }
  }

  $effect(() => {
    connect()
      .then(() => {
        starting = false
        refresh()
      })
      .catch((e) => {
        starting = false
        error = String(e)
      })
    const t = setInterval(refresh, 15000)
    return () => clearInterval(t)
  })

  function go(v: View) {
    view = v
    selected = null
    query = ''
    refresh()
  }

  let searchTimer: ReturnType<typeof setTimeout>
  function onsearch() {
    clearTimeout(searchTimer)
    searchTimer = setTimeout(refresh, 250)
  }

  const count = (c: Category) => status?.counts.by_category?.[c] ?? 0
</script>

<div class="app">
  <nav>
    <div class="brand">Sieve</div>
    {#each CATEGORIES as c}
      <button class="nav" class:active={view === c.key} onclick={() => go(c.key)} title={c.hint}>
        <span class="dot {c.key}"></span>{c.label}<span class="count">{count(c.key) || ''}</span>
      </button>
    {/each}
    <button class="nav" class:active={view === 'all'} onclick={() => go('all')}><span class="dot"></span>All mail</button>
    <div class="sep"></div>
    <button class="nav" class:active={view === 'followups'} onclick={() => go('followups')}>
      <span class="dot"></span>Waiting on others
      {#if status?.counts.followups_due}<span class="count due">{status.counts.followups_due}</span>{/if}
    </button>
    <button class="nav" class:active={view === 'label'} onclick={() => go('label')}><span class="dot"></span>Label mode</button>
    <button class="nav" class:active={view === 'settings'} onclick={() => go('settings')}><span class="dot"></span>Settings</button>
    <div class="grow"></div>
    <div class="state faint">
      {#if starting}Starting Sieve…
      {:else if status}
        {status.status.syncing ? 'Checking mail…' : status.status.last_sync ? `Checked ${when(status.status.last_sync)}` : 'Not synced yet'}
        {#if status.counts.untriaged}<br />{status.counts.untriaged} to sort{/if}
        <br /><span title="Runs on this computer">🔒 {status.status.model}</span>
      {/if}
    </div>
    <button class="ghost small" onclick={() => api.sync().then(refresh)}>Check now</button>
  </nav>

  {#if view === 'followups'}
    <main class="single"><FollowUps onopen={(id) => { view = 'all'; selected = id }} onchange={refresh} /></main>
  {:else if view === 'label'}
    <main class="single"><LabelMode onchange={refresh} /></main>
  {:else if view === 'settings'}
    <main class="single"><Settings {status} onchange={refresh} /></main>
  {:else}
    <section class="list">
      <input class="search" placeholder="Search all mail" bind:value={query} oninput={onsearch} />
      {#if error}<p class="error pad">{error}</p>{/if}
      {#each list as m (m.id)}
        <button class="row" class:active={selected === m.id} class:unread={!m.seen} onclick={() => (selected = m.id)}>
          <div class="line1">
            <span class="from">{sender(m)}</span>
            <span class="faint">{when(m.date)}</span>
          </div>
          <div class="subject">{m.subject || '(no subject)'}</div>
          <div class="line3">
            {#if view === 'all' && (m.label ?? m.category)}<span class="pill {m.label ?? m.category}">{CATEGORIES.find((c) => c.key === (m.label ?? m.category))?.label}</span>{/if}
            <span class="snippet faint">{m.reason || m.snippet}</span>
          </div>
        </button>
      {:else}
        <p class="faint pad">{query ? 'Nothing found.' : 'Nothing here.'}</p>
      {/each}
    </section>
    <main>
      {#if selected !== null}
        <MessageView id={selected} onchange={refresh} />
      {:else}
        <div class="empty faint">
          {CATEGORIES.find((c) => c.key === view)?.hint ?? 'Pick a message'}
        </div>
      {/if}
    </main>
  {/if}
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 210px 360px 1fr;
    height: 100vh;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 10px;
    border-right: 1px solid var(--line);
    background: var(--sunk);
  }
  .brand {
    font-weight: 700;
    font-size: 17px;
    letter-spacing: 0.02em;
    padding: 0 8px 12px;
    color: var(--accent);
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 9px;
    border: none;
    background: transparent;
    text-align: left;
    padding: 6px 8px;
  }
  .nav.active {
    background: var(--panel);
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--line);
    flex-shrink: 0;
  }
  .dot.reply_today {
    background: var(--today);
  }
  .dot.informative {
    background: var(--info);
  }
  .dot.newsletter {
    background: var(--news);
  }
  .dot.invoice {
    background: var(--money);
  }
  .count {
    margin-left: auto;
    font-size: 12px;
    color: var(--muted);
  }
  .count.due {
    color: var(--today);
    font-weight: 700;
  }
  .sep {
    height: 1px;
    background: var(--line);
    margin: 8px 6px;
  }
  .grow {
    flex: 1;
  }
  .state {
    font-size: 12px;
    padding: 6px 8px;
  }
  .small {
    font-size: 12px;
  }
  .list {
    border-right: 1px solid var(--line);
    overflow-y: auto;
    background: var(--panel);
  }
  .search {
    width: calc(100% - 20px);
    margin: 10px;
  }
  .row {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    background: transparent;
    padding: 9px 14px;
  }
  .row.active {
    background: var(--accent-soft);
  }
  .row.unread .from,
  .row.unread .subject {
    font-weight: 600;
  }
  .line1 {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .from,
  .subject,
  .snippet {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .line3 {
    display: flex;
    gap: 6px;
    align-items: center;
    font-size: 12.5px;
    min-width: 0;
  }
  main {
    overflow-y: auto;
    padding: 22px 28px;
  }
  main.single {
    grid-column: 2 / 4;
  }
  .empty {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .pad {
    padding: 0 14px;
  }
</style>
