<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { Search, Plus, Save, ListTree } from "lucide-svelte";
  import { api } from "../lib/api";
  import { emptyRepo, matchesSearch, type Repo } from "../lib/repos";
  import RepoRow from "../components/RepoRow.svelte";
  import RepoFormModal from "../components/RepoFormModal.svelte";
  import Modal from "../components/Modal.svelte";

  let repos: Repo[] = [];
  let loading = true;
  let dirty = false;
  let saving = false;
  let query = "";
  let formTarget: Repo | null = null;
  let deleteTarget: Repo | null = null;
  let errorMessage: string | null = null;
  let filesToDelete: string[] = [];

  async function load() {
    loading = true;
    try {
      repos = await api.invoke<Repo[]>("list_repos");
      filesToDelete = [];
      dirty = false;
    } catch (error) {
      console.error("Failed to load repositories", error);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  $: filtered = repos.filter((r) => matchesSearch(r, query));

  function toggle(repo: Repo) {
    repo.disabled = !repo.disabled;
    repos = repos;
    dirty = true;
  }

  function askDelete(repo: Repo) {
    deleteTarget = repo;
  }

  function confirmDelete() {
    if (!deleteTarget) return;
    const targetFile = deleteTarget.file;
    repos = repos.filter((r) => r !== deleteTarget);
    if (targetFile && !repos.some((r) => r.file === targetFile)) {
      if (!filesToDelete.includes(targetFile)) {
        filesToDelete = [...filesToDelete, targetFile];
      }
    }
    deleteTarget = null;
    dirty = true;
  }

  function openAdd() {
    formTarget = emptyRepo();
  }

  function openEdit(repo: Repo) {
    formTarget = repo;
  }

  function saveForm(draft: Repo) {
    const isNew = !repos.includes(formTarget!);
    if (isNew) {
      repos = [...repos, draft];
    } else {
      repos = repos.map((r) => (r === formTarget ? draft : r));
    }
    formTarget = null;
    dirty = true;
  }

  async function modernize(repo: Repo) {
    try {
      await api.invoke("modernize_repo", { repo });
      await load();
    } catch (error) {
      console.error("Failed to modernize repository", error);
      errorMessage = String(error);
    }
  }

  async function applyChanges() {
    saving = true;
    try {
      await api.invoke("save_repos", { repos, filesToDelete });
      await load();
    } catch (error) {
      console.error("Failed to save repositories", error);
      errorMessage = String(error);
    } finally {
      saving = false;
    }
  }
</script>

<div class="flex items-center gap-2 mb-4">
  <div class="relative flex-1">
    <Search size={14} class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-muted" />
    <input
      bind:value={query}
      placeholder={$_("repositories.searchPlaceholder")}
      class="w-full bg-surface-2 border border-border rounded pl-8 pr-3 py-2 text-[0.82rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60"
    />
  </div>
  <button
    class="flex items-center gap-1.5 px-3 py-2 rounded border border-border text-text hover:bg-surface hover:text-text-bright text-[0.8rem]"
    on:click={openAdd}
  >
    <Plus size={14} />
    {$_("repositories.add")}
  </button>
  <button
    class="flex items-center gap-1.5 px-3 py-2 rounded bg-primary hover:bg-primary-hover text-text-bright text-[0.8rem] font-semibold disabled:opacity-40 disabled:cursor-not-allowed"
    disabled={!dirty || saving}
    on:click={applyChanges}
  >
    <Save size={14} />
    {saving ? $_("repositories.applying") : $_("repositories.apply")}
  </button>
</div>

{#if loading}
  <p class="text-text-muted text-[0.85rem]">{$_("common.loading")}</p>
{:else if filtered.length === 0}
  <div class="h-full flex flex-col items-center justify-center gap-2 text-center py-10">
    <ListTree size={56} class="text-text-muted" />
    <p class="text-[0.85rem] text-text-muted">{$_("repositories.empty")}</p>
  </div>
{:else}
  <div class="flex flex-col gap-2">
    {#each filtered as repo (repo.file + repo.uri + repo.distribution + (repo.block ?? repo.components))}
      <RepoRow {repo} onToggle={toggle} onEdit={openEdit} onDelete={askDelete} onModernize={modernize} />
    {/each}
  </div>
{/if}

{#if formTarget}
  <RepoFormModal repo={formTarget} onSave={saveForm} onClose={() => (formTarget = null)} />
{/if}

{#if deleteTarget}
  <Modal variant="danger" title={$_("repositories.deleteConfirmTitle")} confirmLabel={$_("repositories.delete")} onClose={() => (deleteTarget = null)} onConfirm={confirmDelete}>
    <p>{$_("repositories.deleteConfirmBody")}</p>
  </Modal>
{/if}

{#if errorMessage}
  <Modal variant="info" title={$_("common.errorTitle")} onClose={() => (errorMessage = null)}>
    <p>{errorMessage}</p>
  </Modal>
{/if}
