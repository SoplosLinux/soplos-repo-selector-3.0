<script lang="ts">
  import { _ } from "svelte-i18n";
  import { Pencil, Trash2, Wand2 } from "lucide-svelte";
  import type { Repo } from "../lib/repos";

  export let repo: Repo;
  export let onToggle: (r: Repo) => void;
  export let onEdit: (r: Repo) => void;
  export let onDelete: (r: Repo) => void;
  export let onModernize: (r: Repo) => void;

  $: isLegacy = repo.format === "legacy";
</script>

<div class="border border-border rounded p-3 flex items-center gap-3 {repo.disabled ? 'opacity-50' : ''}">
  <button
    class="relative w-[32px] h-[18px] rounded-full transition-colors shrink-0 {!repo.disabled ? 'bg-primary' : 'bg-surface-2'}"
    on:click={() => onToggle(repo)}
    aria-pressed={!repo.disabled}
    title={repo.disabled ? $_("repositories.enable") : $_("repositories.disable")}
  >
    <span
      class="absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-text-bright transition-transform {!repo.disabled ? 'translate-x-[14px]' : ''}"
    ></span>
  </button>

  <div class="min-w-0 flex-1">
    <div class="text-[0.85rem] font-semibold text-text-bright truncate">{repo.uri}</div>
    <div class="text-[0.72rem] text-text-muted truncate">
      {repo.type} · {repo.distribution}{repo.components ? ` · ${repo.components}` : ""}
    </div>
    <div class="text-[0.66rem] text-text-muted truncate mt-0.5">{repo.file}</div>
  </div>

  <div class="flex items-center gap-1 shrink-0">
    {#if isLegacy}
      <button
        class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-primary"
        title={$_("repositories.modernize")}
        on:click={() => onModernize(repo)}
      >
        <Wand2 size={15} />
      </button>
    {/if}
    <button class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-text-bright" title={$_("repositories.edit")} on:click={() => onEdit(repo)}>
      <Pencil size={15} />
    </button>
    <button class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-danger" title={$_("repositories.delete")} on:click={() => onDelete(repo)}>
      <Trash2 size={15} />
    </button>
  </div>
</div>
