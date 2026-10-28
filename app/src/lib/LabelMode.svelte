<script lang="ts">
  import { api, CATEGORIES, sender, type Message } from './api'

  let { onchange }: { onchange: () => void } = $props()
  let msg = $state<Message | null>(null)
  let labelled = $state(0)
  let done = $state(false)
  let error = $state('')

  async function next() {
    try {
      const r = await api.nextUnlabelled()
      msg = r.message
      labelled = r.labelled
      done = !r.message
    } catch (e) {
      error = String(e)
    }
  }
  $effect(() => {
    next()
  })

  async function pick(i: number) {
    if (!msg) return
    await api.label(msg.id, CATEGORIES[i].key)
    onchange()
    await next()
  }

  function onkey(e: KeyboardEvent) {
    const n = Number(e.key)
    if (n >= 1 && n <= 4) pick(n - 1)
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="wrap">
  <h2>Label your mail</h2>
  <p class="muted">
    Your labels are the truth Sieve's triage is measured against (<code>python -m sieve_eval.triage mine</code>). They stay on this computer.
    Press <kbd>1</kbd>–<kbd>4</kbd>. {labelled} labelled so far.
  </p>
  {#if error}<p class="error">{error}</p>{/if}
  {#if done}
    <p>All your messages are labelled. Thank you!</p>
  {:else if msg}
    <div class="card">
      <div class="head">
        <strong>{sender(msg)}</strong> <span class="faint">&lt;{msg.from_addr}&gt;</span>
        <div class="subject">{msg.subject}</div>
      </div>
      <div class="body">{msg.body.slice(0, 1800)}{msg.body.length > 1800 ? '…' : ''}</div>
    </div>
    <div class="choices">
      {#each CATEGORIES as c, i}
        <button onclick={() => pick(i)}>
          <kbd>{i + 1}</kbd> <span class="pill {c.key}">{c.label}</span>
          <span class="faint small">{c.hint}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .wrap {
    max-width: 760px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 19px;
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 14px 18px;
    margin: 12px 0;
  }
  .subject {
    font-size: 16px;
    margin: 4px 0 10px;
  }
  .body {
    white-space: pre-wrap;
    max-height: 340px;
    overflow: auto;
    overflow-wrap: anywhere;
  }
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .choices button {
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    padding: 10px 12px;
  }
  kbd {
    font-family: inherit;
    font-size: 11px;
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 0 5px;
    background: var(--sunk);
  }
  .small {
    font-size: 12px;
  }
</style>
