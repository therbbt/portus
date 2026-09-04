<script lang="ts">
  import type { Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import Dialog from "./Dialog.svelte";

  export let update: Update;
  // Covers both "Not now" and closing the dialog any other way (Escape,
  // outside click) - all count as the user having seen and passed on this
  // version, so the reminder doesn't reappear until a newer one ships.
  export let onDismiss: () => void;

  let installing = false;
  let progressLabel = "";
  let errorMessage = "";

  const formattedDate = (() => {
    if (!update.date) return null;
    const parsed = new Date(update.date);
    if (Number.isNaN(parsed.getTime())) return null;
    return parsed.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" });
  })();

  async function install() {
    installing = true;
    errorMessage = "";
    progressLabel = "Downloading update…";
    let totalBytes = 0;
    let downloadedBytes = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          progressLabel = totalBytes > 0 ? `Downloading update… ${Math.min(100, Math.round((downloadedBytes / totalBytes) * 100))}%` : "Downloading update…";
        } else if (event.event === "Finished") {
          progressLabel = "Installing…";
        }
      });
      await relaunch();
    } catch (err) {
      installing = false;
      errorMessage = err instanceof Error ? err.message : "Failed to install the update. Please try again later.";
    }
  }
</script>

<Dialog label="Update available" width="380px" on:cancel={() => !installing && onDismiss()}>
  <h2 class="title">Update available</h2>

  <p class="version-line">
    <strong>Portus {update.version}</strong>
    <span class="muted">you have {update.currentVersion}</span>
  </p>
  {#if formattedDate}
    <p class="date">Released {formattedDate}</p>
  {/if}

  {#if update.body}
    <!-- Plain text, not rendered as markdown - GitHub's auto-generated
         release notes are still readable enough as-is (a stray "##"/"-"
         here and there) without pulling in a markdown renderer just for
         this. -->
    <div class="notes">{update.body}</div>
  {:else}
    <p class="notes empty">No release notes provided.</p>
  {/if}

  {#if errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}

  <div class="actions">
    <button class="btn" on:click={onDismiss} disabled={installing}>Not now</button>
    <button class="btn primary" on:click={install} disabled={installing}>
      {installing ? progressLabel : "Install and Restart"}
    </button>
  </div>
</Dialog>

<style>
  .title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .version-line {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin: 0;
    font-size: 0.8rem;
    color: var(--fg-primary);
  }

  .muted {
    font-size: 0.7rem;
    color: var(--fg-secondary);
  }

  .date {
    margin: 0;
    font-size: 0.7rem;
    color: var(--fg-secondary);
  }

  .notes {
    max-height: 40vh;
    overflow-y: auto;
    background: var(--surface-1);
    border-radius: var(--radius-md);
    padding: 0.6rem 0.7rem;
    font-size: 0.75rem;
    line-height: 1.5;
    color: var(--fg-primary);
    white-space: pre-wrap;
  }

  p.notes.empty {
    color: var(--fg-tertiary);
    margin: 0;
  }

  .error {
    margin: 0;
    font-size: 0.72rem;
    color: var(--status-error);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .btn {
    border: none;
    border-radius: var(--radius-md);
    background: var(--surface-3);
    color: var(--fg-primary);
    font-size: 0.78rem;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
  }
  .btn:hover {
    background: var(--surface-4);
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  .btn.primary:hover {
    filter: brightness(1.08);
  }
  .btn.primary:disabled,
  .btn:disabled {
    background: var(--surface-4);
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
</style>
