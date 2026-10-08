<script lang="ts">
  import { _ } from "svelte-i18n";
  import Modal from "./Modal.svelte";
  import type { Repo } from "../lib/repos";

  export let repo: Repo;
  export let onSave: (r: Repo) => void;
  export let onClose: () => void;

  let draft: Repo = { ...repo };
  $: isEdit = repo.uri !== "";

  function save() {
    onSave(draft);
  }
</script>

<Modal
  variant="confirm"
  title={isEdit ? $_("repositories.editTitle") : $_("repositories.addTitle")}
  confirmLabel={$_("common.save")}
  {onClose}
  onConfirm={save}
  confirmDisabled={!draft.uri || !draft.distribution}
>
  <div class="flex flex-col gap-3">
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("repositories.type")}</span>
      <select bind:value={draft.type} class="bg-surface-2 border border-border rounded px-2.5 py-2 text-[0.85rem] text-text-bright">
        <option value="deb">deb</option>
        <option value="deb-src">deb-src</option>
      </select>
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("repositories.uri")}</span>
      <input bind:value={draft.uri} placeholder="http://deb.debian.org/debian" class="bg-surface-2 border border-border rounded px-2.5 py-2 text-[0.85rem] text-text-bright" />
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("repositories.distribution")}</span>
      <input bind:value={draft.distribution} placeholder="trixie" class="bg-surface-2 border border-border rounded px-2.5 py-2 text-[0.85rem] text-text-bright" />
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("repositories.components")}</span>
      <input bind:value={draft.components} placeholder="main contrib non-free" class="bg-surface-2 border border-border rounded px-2.5 py-2 text-[0.85rem] text-text-bright" />
    </label>
    <label class="flex flex-col gap-1">
      <span class="text-[0.78rem] text-text-muted">{$_("repositories.signedBy")}</span>
      <input bind:value={draft.signedBy} placeholder="/usr/share/keyrings/example.gpg" class="bg-surface-2 border border-border rounded px-2.5 py-2 text-[0.85rem] text-text-bright" />
    </label>
  </div>
</Modal>
