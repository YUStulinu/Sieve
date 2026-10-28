<script lang="ts">
  import { api, when, type FollowUp } from './api'

  let { onopen, onchange }: { onopen: (id: number) => void; onchange: () => void } = $props()
  let items = $state<FollowUp[]>([])
  let error = $state('')

  async function load() {
    try {
      items = await api.followups('waiting')
    } catch (e) {
      error = String(e)
    }
  }
  $effect(() => {
    load()
  })

  async function act(f: FollowUp, action: 'dismiss' | 'done' | 'snooze') {
    await api.followupAction(f.id, action, action === 'snooze' ? 2 : undefined)
    await load()
    onchange()
  }

  const due = (f: FollowUp) => {
    const days = Math.round((f.due - Date.now() / 1000) / 86400)
    if (f.overdue) return days === 0 ? 'due today' : `${-days} day${-days === 1 ? '' : 's'} overdue`
    return `remind in ${days} day${days === 1 ? '' : 's'}`
  }
</script>

<div class="wrap">
  <h2>Waiting for an answer</h2>
  <p class="muted">Messages you sent that asked for something. When someone answers in the thread, the follow-up closes by itself.</p>
  {#if error}<p class="error">{error}</p>{/if}
  {#if !items.length}
    <p class="faint">Nobody owes you an answer right now.</p>
  {/if}
  {#each items as f (f.id)}
    <div class="item" class:overdue={f.overdue}>
      <button class="ghost open" onclick={() => onopen(f.message_id)}>
        <div><strong>{f.waiting_on}</strong> <span class="muted">— {f.what || f.subject}</span></div>
        <div class="faint small">“{f.subject}” · sent {when(f.sent)} · {due(f)}</div>
      </button>
      <div class="actions">
        <button onclick={() => act(f, 'snooze')}>Remind me in 2 days</button>
        <button onclick={() => act(f, 'done')}>Answered</button>
        <button class="ghost" onclick={() => act(f, 'dismiss')}>Stop waiting</button>
      </div>
    </div>
  {/each}
</div>

<style>
  .wrap {
    max-width: 760px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 19px;
  }
  .item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    border: 1px solid var(--line);
    background: var(--panel);
    border-radius: var(--radius);
    padding: 8px 10px;
    margin-bottom: 8px;
  }
  .item.overdue {
    border-left: 4px solid var(--today);
  }
  .open {
    text-align: left;
    flex: 1;
    padding: 2px 4px;
  }
  .small {
    font-size: 12px;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
</style>
