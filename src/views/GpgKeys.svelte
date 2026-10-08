<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Upload, KeyRound } from "lucide-svelte";
  import { api } from "../lib/api";
  import type { GpgKey } from "../lib/gpg";
  import GpgKeyRow from "../components/GpgKeyRow.svelte";
  import Modal from "../components/Modal.svelte";

  let keys: GpgKey[] = [];
  let loading = true;
  let errorMessage: string | null = null;

  $: unusedCount = keys.filter((k) => k.unused).length;

  async function load() {
    loading = true;
    try {
      keys = await api.invoke<GpgKey[]>("list_gpg_keys");
    } catch (error) {
      console.error("Failed to load GPG keys", error);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function doImport() {
    try {
      const path = await api.invoke<string | null>("pick_gpg_key_file");
      if (!path) return;
      await api.invoke("import_gpg_key", { sourcePath: path, keyName: null });
      await load();
    } catch (error) {
      console.error("Failed to import key", error);
      errorMessage = String(error);
    }
  }

  async function doExport(key: GpgKey) {
    try {
      const dest = await api.invoke<string | null>("pick_gpg_export_path", { suggestedName: key.name });
      if (!dest) return;
      await api.invoke("export_gpg_key", { srcPath: key.path, dstPath: dest });
    } catch (error) {
      console.error("Failed to export key", error);
      errorMessage = String(error);
    }
  }

  async function doDelete(key: GpgKey) {
    try {
      await api.invoke("delete_gpg_key", { keyPath: key.path, force: false });
      await load();
    } catch (error) {
      errorMessage = String(error);
    }
  }
</script>

<div class="flex items-center justify-end mb-4">
  <button
    class="flex items-center gap-1.5 px-3 py-2 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.8rem] font-semibold"
    on:click={doImport}
  >
    <Upload size={14} />
    {$_("gpg.import")}
  </button>
</div>

{#if !loading && unusedCount > 0}
  <div class="mb-4 px-3 py-2.5 rounded border border-warning/30 bg-warning/10 text-[0.8rem] text-text">
    {$_("gpg.unusedSummary", { values: { count: unusedCount } })}
  </div>
{/if}

{#if loading}
  <p class="text-text-muted text-[0.85rem]">{$_("common.loading")}</p>
{:else if keys.length === 0}
  <div class="h-full flex flex-col items-center justify-center gap-2 text-center py-10">
    <KeyRound size={56} class="text-text-muted" />
    <p class="text-[0.85rem] text-text-muted">{$_("gpg.empty")}</p>
  </div>
{:else}
  <div class="flex flex-col gap-2">
    {#each keys as key (key.path)}
      <GpgKeyRow gpgKey={key} onExport={doExport} onDelete={doDelete} />
    {/each}
  </div>
{/if}

{#if errorMessage}
  <Modal variant="info" title={$_("common.errorTitle")} onClose={() => (errorMessage = null)}>
    <p>{errorMessage}</p>
  </Modal>
{/if}
