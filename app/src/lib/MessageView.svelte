<script lang="ts">
  import { api, CATEGORIES, confirmationFor, sender, when, type Category, type Detail, type Draft } from './api'

  let { id, onchange }: { id: number; onchange: () => void } = $props()

  let detail = $state<Detail | null>(null)
  let error = $state('')
  let busy = $state('')
  let draft = $state<Draft | null>(null)
  let text = $state('')
  let confirming = $state(false)

  async function load() {
    error = ''
    try {
      detail = await api.message(id)
      draft = detail.drafts.find((d) => d.status !== 'sent') ?? null
      text = draft?.body ?? ''
    } catch (e) {
      error = String(e)
    }
  }

  $effect(() => {
    id
    confirming = false
    load()
  })

  async function run(what: string, f: () => Promise<unknown>) {
    busy = what
    error = ''
    try {
      await f()
    } catch (e) {
      error = String(e)
    } finally {
      busy = ''
    }
  }

  const propose = () =>
    run('Writing a draft…', async () => {
      draft = await api.draft(id)
      text = draft.body
    })

  const keepEdits = async () => {
    if (draft && text !== draft.body) draft = await api.editDraft(draft.id, text)
  }

  const saveToDrafts = () =>
    run('Saving to Drafts…', async () => {
      await keepEdits()
      draft = await api.saveDraft(draft!.id)
    })

  const send = () =>
    run('Sending…', async () => {
      await keepEdits()
      draft = await api.sendDraft(draft!.id, confirmationFor(draft!))
      confirming = false
      onchange()
    })

  const discard = () =>
    run('', async () => {
      await api.discardDraft(draft!.id)
      draft = null
      text = ''
    })

  const summarize = () =>
    run('Summarizing…', async () => {
      const s = await api.summarize(detail!.message.thread_id)
      detail = { ...detail!, summary: s }
    })

  const relabel = (c: Category) =>
    run('', async () => {
      await api.label(id, c)
      detail = { ...detail!, label: c }
      onchange()
    })
</script>

{#if error}<p class="error">{error}</p>{/if}
{#if detail}
  {@const m = detail.message}
  <article>
    <header>
      <h2>{m.subject || '(no subject)'}</h2>
      <div class="meta">
        <span><strong>{sender(m)}</strong> <span class="faint">&lt;{m.from_addr}&gt;</span></span>
        <span class="faint">{new Date(m.date * 1000).toLocaleString()}</span>
      </div>
      {#if detail.category}
        <div class="verdict">
          <span class="pill {detail.label ?? detail.category}">{CATEGORIES.find((c) => c.key === (detail!.label ?? detail!.category))?.label}</span>
          {#if detail.reason && !detail.label}<span class="muted">{detail.reason}</span>{/if}
          <span class="faint relabel">
            Wrong pile?
            {#each CATEGORIES as c}
              {#if c.key !== (detail.label ?? detail.category)}
                <button class="ghost link" onclick={() => relabel(c.key)}>{c.label}</button>
              {/if}
            {/each}
          </span>
        </div>
      {/if}
    </header>

    {#if detail.thread.length > 1}
      <section class="summary">
        {#if detail.summary}
          <p>{detail.summary.summary.summary}</p>
          {#if detail.summary.summary.decisions?.length}
            <p class="faint small">Decided</p>
            <ul>{#each detail.summary.summary.decisions as d}<li>{d}</li>{/each}</ul>
          {/if}
          {#if detail.summary.summary.open_questions?.length}
            <p class="faint small">Open</p>
            <ul>{#each detail.summary.summary.open_questions as d}<li>{d}</li>{/each}</ul>
          {/if}
          {#if detail.summary.summary.waiting_on}<p class="muted small">Next: {detail.summary.summary.waiting_on}</p>{/if}
          {#if detail.summary.messages < detail.thread.length}
            <button class="ghost link" onclick={summarize} disabled={!!busy}>Update the summary ({detail.thread.length} messages now)</button>
          {/if}
        {:else}
          <button onclick={summarize} disabled={!!busy}>Summarize this thread ({detail.thread.length} messages)</button>
        {/if}
      </section>
    {/if}

    <section class="thread">
      {#each detail.thread as t (t.brief.id)}
        <div class="msg" class:current={t.brief.id === id} class:mine={t.brief.outgoing}>
          <div class="msg-head">
            <strong>{t.brief.outgoing ? 'You' : sender(t.brief)}</strong>
            <span class="faint">{when(t.brief.date)}</span>
          </div>
          <div class="body">{t.body}</div>
          {#if t.brief.attachments.length}<div class="faint small">📎 {t.brief.attachments.join(', ')}</div>{/if}
        </div>
      {/each}
    </section>

    {#if !m.outgoing}
      <section class="draft">
        {#if !draft}
          <button class="primary" onclick={propose} disabled={!!busy}>Draft a reply in my style</button>
        {:else if draft.status === 'sent'}
          <p class="muted">Sent to {draft.to.join(', ')}.</p>
        {:else}
          <div class="draft-head">
            <span class="muted">To {draft.to.join(', ')} · {draft.subject}</span>
            <span class="faint small">{draft.status === 'saved' ? 'Saved in your Drafts folder' : 'Proposal — edit freely; nothing is sent without you'}</span>
          </div>
          <textarea bind:value={text} rows="9"></textarea>
          {#if confirming}
            <div class="confirm" role="alertdialog" aria-label="Confirm sending">
              <p>Send this reply to <strong>{draft.to.join(', ')}</strong> now?</p>
              <div class="row">
                <button class="primary" onclick={send} disabled={!!busy || !text.trim()}>Yes, send it</button>
                <button onclick={() => (confirming = false)}>Not yet</button>
              </div>
            </div>
          {:else}
            <div class="row">
              <button onclick={saveToDrafts} disabled={!!busy}>Save to Drafts</button>
              <button class="primary" onclick={() => (confirming = true)} disabled={!!busy || !text.trim()}>Send…</button>
              <button class="ghost" onclick={propose} disabled={!!busy}>Write another</button>
              <button class="ghost danger" onclick={discard} disabled={!!busy}>Discard</button>
            </div>
          {/if}
        {/if}
        {#if busy}<p class="muted small">{busy}</p>{/if}
      </section>
    {/if}
  </article>
{:else if !error}
  <p class="muted">Loading…</p>
{/if}

<style>
  article {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 820px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 19px;
    font-weight: 600;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .verdict {
    margin-top: 10px;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .relabel {
    font-size: 12px;
    margin-left: auto;
  }
  .link {
    padding: 0 4px;
    font-size: 12px;
    text-decoration: underline;
    color: var(--muted);
  }
  .summary {
    background: var(--accent-soft);
    border-radius: var(--radius);
    padding: 12px 16px;
  }
  .summary p {
    margin: 0 0 6px;
  }
  .summary ul {
    margin: 0 0 8px;
    padding-left: 18px;
  }
  .small {
    font-size: 12px;
  }
  .thread {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .msg {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 12px 16px;
  }
  .msg.mine {
    background: var(--sunk);
  }
  .msg.current {
    border-color: var(--faint);
  }
  .msg-head {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
  }
  .body {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .draft {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 24px;
  }
  .draft-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }
  textarea {
    width: 100%;
    resize: vertical;
    line-height: 1.5;
  }
  .row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .confirm {
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    padding: 10px 14px;
    background: var(--accent-soft);
  }
  .confirm p {
    margin: 0 0 8px;
  }
</style>
