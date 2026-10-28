<script lang="ts">
  import { api, type Account, type Status } from './api'

  let { status, onchange }: { status: Status | null; onchange: () => void } = $props()
  let accounts = $state<Account[]>([])
  let style = $state<Awaited<ReturnType<typeof api.style>>>([])
  let error = $state('')
  let busy = $state(false)
  let form = $state({
    name: '',
    email: '',
    username: '',
    imap_host: '',
    imap_port: 993,
    imap_security: 'tls',
    smtp_host: '',
    smtp_port: 587,
    smtp_security: 'starttls',
    password: '',
  })

  async function load() {
    try {
      accounts = await api.accounts()
      style = await api.style()
    } catch (e) {
      error = String(e)
    }
  }
  $effect(() => {
    load()
  })

  async function add(e: Event) {
    e.preventDefault()
    busy = true
    error = ''
    try {
      await api.addAccount({ ...form, username: form.username || form.email })
      form.password = ''
      await load()
      onchange()
    } catch (err) {
      error = String(err)
    } finally {
      busy = false
    }
  }

  async function remove(a: Account) {
    if (!confirm(`Remove ${a.email} from Sieve? Its local copy goes; the mailbox on the server stays as it is.`)) return
    await api.deleteAccount(a.id)
    await load()
    onchange()
  }
</script>

<div class="wrap">
  <h2>Accounts</h2>
  {#each accounts as a (a.id)}
    <div class="acct">
      <div><strong>{a.name}</strong> &lt;{a.email}&gt; <span class="faint">{a.imap_host}:{a.imap_port}</span></div>
      <button class="ghost danger" onclick={() => remove(a)}>Remove</button>
    </div>
  {:else}
    <p class="muted">No account yet.</p>
  {/each}

  <form onsubmit={add}>
    <h3>Add an account</h3>
    <p class="faint small">
      The password goes to Windows Credential Manager, never to a file. For Gmail and Outlook use an app password.
    </p>
    <div class="grid">
      <label>Name <input bind:value={form.name} required placeholder="Ana Popescu" /></label>
      <label>Email <input bind:value={form.email} required type="email" /></label>
      <label>IMAP server <input bind:value={form.imap_host} required placeholder="imap.gmail.com" /></label>
      <label>Port <input bind:value={form.imap_port} type="number" /></label>
      <label>IMAP security
        <select bind:value={form.imap_security}><option value="tls">TLS</option><option value="none">None (this computer only)</option></select>
      </label>
      <label>SMTP server <input bind:value={form.smtp_host} required placeholder="smtp.gmail.com" /></label>
      <label>Port <input bind:value={form.smtp_port} type="number" /></label>
      <label>SMTP security
        <select bind:value={form.smtp_security}><option value="starttls">STARTTLS</option><option value="tls">TLS</option><option value="none">None (this computer only)</option></select>
      </label>
      <label>Login (if not the email) <input bind:value={form.username} /></label>
      <label>Password <input bind:value={form.password} type="password" required autocomplete="off" /></label>
    </div>
    <button class="primary" disabled={busy}>{busy ? 'Checking the login…' : 'Add'}</button>
    {#if error}<p class="error">{error}</p>{/if}
  </form>

  <h2>How you write</h2>
  <p class="muted small">Learned from your sent mail; drafts follow it.</p>
  {#each style as s}
    <div class="acct col">
      <strong>{s.account}</strong>
      <span class="muted">{s.style.messages} sent messages · usually {s.style.median_words} words</span>
      {#each Object.entries(s.style.by_lang) as [lang, ls]}
        <span>
          <span class="faint">{lang === 'ro' ? 'Romanian' : 'English'} ({ls.messages}):</span>
          {ls.greetings.length ? ls.greetings.map((g) => g.replace('{name}', '…')).join(' · ') : 'no greeting'} —
          {ls.signoffs.length ? ls.signoffs[0].replace('\n', ' ') : 'no sign-off'}
        </span>
      {/each}
    </div>
  {/each}

  <h2>Model</h2>
  {#if status}
    <p>
      {status.status.model} · {status.counts.messages} messages, {status.counts.untriaged} waiting for triage
      {#if status.status.last_error}<br /><span class="error">{status.status.last_error}</span>{/if}
    </p>
  {/if}
</div>

<style>
  .wrap {
    max-width: 760px;
    padding-bottom: 32px;
  }
  h2 {
    margin: 16px 0 6px;
    font-size: 19px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 15px;
  }
  .acct {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 8px 12px;
    margin-bottom: 6px;
  }
  .acct.col {
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }
  form {
    margin-top: 14px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 14px;
    margin: 10px 0 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
</style>
