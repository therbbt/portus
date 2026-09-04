<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Group } from "../bridge";
  import Dialog from "./Dialog.svelte";
  import FolderSelect from "./FolderSelect.svelte";

  export let groups: Group[] = [];
  export let suggestedName = "";

  const dispatch = createEventDispatcher<{
    save: { name: string; groupId: string | null };
    cancel: void;
  }>();

  let name = suggestedName;
  let groupId = "";

  $: canSave = name.trim().length > 0;

  function submit() {
    if (!canSave) return;
    dispatch("save", { name: name.trim(), groupId: groupId || null });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<Dialog label="Save session" width="320px" on:cancel={() => dispatch("cancel")}>
  <h2 class="title">Save session</h2>

  <label class="field">
    <span>Name</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input type="text" bind:value={name} autofocus />
  </label>

  <label class="field">
    <span>Folder</span>
    <FolderSelect {groups} bind:value={groupId} />
  </label>

  <div class="actions">
    <button class="btn" on:click={() => dispatch("cancel")}>Cancel</button>
    <button class="btn primary" disabled={!canSave} on:click={submit}>Save</button>
  </div>
</Dialog>

<style>
  .title {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.72rem;
    color: var(--fg-secondary);
  }

  input {
    background: var(--surface-1);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0.4rem 0.5rem;
    color: var(--fg-primary);
    font-size: 0.8rem;
  }
  input:focus-visible {
    box-shadow: 0 0 0 2px var(--accent);
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
  .btn.primary:disabled {
    background: var(--surface-4);
    color: var(--fg-disabled);
    cursor: not-allowed;
  }
</style>
