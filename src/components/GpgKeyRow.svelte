<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Download, Trash2, KeyRound } from "lucide-svelte";
  import { api } from "../lib/api";
  import type { GpgKey, GpgKeyDetails } from "../lib/gpg";

  export let gpgKey: GpgKey;
  export let onExport: (k: GpgKey) => void;
  export let onDelete: (k: GpgKey) => void;

  let details: GpgKeyDetails | null = null;
  let confirmArmed = false;
  let armTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    try {
      details = await api.invoke<GpgKeyDetails | null>("get_gpg_key_details", { keyPath: gpgKey.path });
    } catch (error) {
      console.error("Failed to read key details", error);
    }
  });

  // Two-step inline confirm: first click arms a 5s window (matching the
  // 2.x GPG tab), second click while armed actually deletes.
  function handleDeleteClick() {
    if (confirmArmed) {
      if (armTimer) clearTimeout(armTimer);
      confirmArmed = false;
      onDelete(gpgKey);
      return;
    }
    confirmArmed = true;
    armTimer = setTimeout(() => {
      confirmArmed = false;
    }, 5000);
  }
</script>

<div class="border border-border rounded p-3 flex items-center gap-3">
  <div class="w-9 h-9 rounded bg-surface-2 grid place-items-center shrink-0">
    <KeyRound size={16} class="text-text-muted" />
  </div>
  <div class="min-w-0 flex-1">
    <div class="flex items-center gap-1.5">
      <span class="text-[0.85rem] font-semibold text-text-bright truncate">{gpgKey.name}</span>
      {#if gpgKey.unused}
        <span class="text-[0.62rem] px-1.5 py-0.5 rounded bg-warning/15 text-warning shrink-0" title={$_("gpg.unusedHint")}>
          {$_("gpg.unused")}
        </span>
      {/if}
    </div>
    <div class="text-[0.72rem] text-text-muted truncate">{gpgKey.description}</div>
    {#if details}
      <div class="text-[0.66rem] text-text-muted truncate mt-0.5">
        {#if details.fingerprint}{$_("gpg.fingerprint")}: {details.fingerprint}{/if}
        {#if details.expiry}{"  ·  "}{$_("gpg.expires")}: {details.expiry.slice(0, 10)}{/if}
      </div>
    {/if}
  </div>
  <div class="flex items-center gap-1 shrink-0">
    <button class="p-1.5 rounded text-text-muted hover:bg-surface-2 hover:text-text-bright" title={$_("gpg.export")} on:click={() => onExport(gpgKey)}>
      <Download size={15} />
    </button>
    <button
      class="px-2 py-1.5 rounded text-[0.72rem] font-semibold {confirmArmed ? 'bg-danger text-text-bright' : 'text-text-muted hover:bg-surface-2 hover:text-danger'}"
      on:click={handleDeleteClick}
    >
      {#if confirmArmed}
        {$_("gpg.confirmDelete")}
      {:else}
        <Trash2 size={15} />
      {/if}
    </button>
  </div>
</div>
